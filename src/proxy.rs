use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures::StreamExt;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

const MAX_LINE_BYTES: usize = 8 * 1024;
const MAX_HEADERS: usize = 64;
const MAX_MANIFEST_BYTES: usize = 10 * 1024 * 1024;
const CHUNK_IDLE_TIMEOUT_SECS: u64 = 60;
const WATCHDOG_IDLE_SECS: u64 = 600;

struct ConnectionGuard {
    conns: Arc<AtomicUsize>,
    activity: Arc<Mutex<Instant>>,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.conns.fetch_sub(1, Ordering::Relaxed);
        if let Ok(mut lock) = self.activity.lock() {
            *lock = Instant::now();
        }
    }
}

pub fn spawn_sidecar(
    target_url: &str,
    headers: &[(String, String)],
    subtitle_url: Option<&str>,
    max_height: Option<u64>,
) -> Result<String, String> {
    let exe = std::env::current_exe()
        .ok()
        .or_else(|| std::env::args().next().map(PathBuf::from))
        .ok_or_else(|| "unable to locate current executable".to_string())?;

    let headers_json = serde_json::to_string(headers).unwrap_or_else(|_| "[]".to_string());
    let sub_arg = subtitle_url.unwrap_or("");
    let max_h_arg = max_height.map(|h| h.to_string()).unwrap_or_default();

    let mut cmd = Command::new(exe);
    cmd.args(["--proxy-for-vlc", target_url, &headers_json, sub_arg, &max_h_arg]);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to spawn proxy sidecar: {e}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to capture sidecar stdout".to_string())?;

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        use std::io::BufRead;
        let mut line = String::new();
        let _ = std::io::BufReader::new(stdout).read_line(&mut line);
        let _ = tx.send(line);
    });

    let line = rx.recv_timeout(Duration::from_secs(8)).map_err(|_| {
        let _ = child.kill();
        let _ = child.wait();
        "proxy sidecar timed out waiting for PORT line".to_string()
    })?;

    let port: u16 = line
        .trim()
        .strip_prefix("PORT ")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| {
            let _ = child.kill();
            let _ = child.wait();
            format!("proxy sidecar returned unexpected output: {line:?}")
        })?;
    std::mem::forget(child);

    let proxy_path = if let Some(rest) = target_url.strip_prefix("https://") {
        format!("/https/{rest}")
    } else if let Some(rest) = target_url.strip_prefix("http://") {
        format!("/http/{rest}")
    } else {
        format!("/https/{target_url}")
    };

    Ok(format!("http://127.0.0.1:{port}{proxy_path}"))
}

#[derive(Clone, Default)]
struct ProxyCache {
    manifest: Arc<tokio::sync::RwLock<Option<String>>>,
    segments: Arc<tokio::sync::RwLock<SegmentCache>>,
}

struct SegmentCache {
    max_entries: usize,
    order: std::collections::VecDeque<String>,
    entries: std::collections::HashMap<String, bytes::Bytes>,
}

impl Default for SegmentCache {
    fn default() -> Self {
        Self::new(32)
    }
}

impl SegmentCache {
    fn new(max_entries: usize) -> Self {
        Self {
            max_entries,
            order: std::collections::VecDeque::new(),
            entries: std::collections::HashMap::new(),
        }
    }

    fn get(&self, key: &str) -> Option<bytes::Bytes> {
        self.entries.get(key).cloned()
    }

    fn insert(&mut self, key: String, data: bytes::Bytes) {
        if self.entries.contains_key(&key) {
            self.entries.insert(key, data);
            return;
        }
        if self.entries.len() >= self.max_entries {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            }
        }
        self.order.push_back(key.clone());
        self.entries.insert(key, data);
    }
}

pub async fn run_sidecar(
    target_url: String,
    headers: Vec<(String, String)>,
    subtitle_url: Option<String>,
    max_height: Option<u64>,
) {
    let client = crate::net::streaming_client_builder()
        .connect_timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_default();

    let listener = match TcpListener::bind("127.0.0.1:0").await {
        Ok(l) => l,
        Err(_) => return,
    };

    let port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(_) => return,
    };

    println!("PORT {port}");
    use std::io::Write;
    let _ = std::io::stdout().flush();

    let active_connections = Arc::new(AtomicUsize::new(0));
    let last_activity = Arc::new(Mutex::new(Instant::now()));

    let watchdog_conns = Arc::clone(&active_connections);
    let watchdog_activity = Arc::clone(&last_activity);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(15)).await;
            let conns = watchdog_conns.load(Ordering::Relaxed);
            let elapsed = {
                let lock = watchdog_activity.lock().unwrap();
                lock.elapsed()
            };
            if conns == 0 && elapsed > Duration::from_secs(WATCHDOG_IDLE_SECS) {
                std::process::exit(0);
            }
        }
    });

    let target_host = extract_host_authority(&target_url);
    let proxy_cache = ProxyCache::default();

    // Warm-prefetch opening manifest and initial segments in background
    let warmup_client = client.clone();
    let warmup_headers = headers.clone();
    let warmup_target = target_url.clone();
    let warmup_cache = proxy_cache.clone();
    let warmup_target_host = target_host.clone();
    let warmup_sub = subtitle_url.clone();
    tokio::spawn(async move {
        warmup_dash_session(
            &warmup_client,
            &warmup_target,
            &warmup_headers,
            port,
            warmup_target_host.as_deref(),
            warmup_sub.as_deref(),
            max_height,
            &warmup_cache,
        )
        .await;
    });

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(val) => val,
            Err(err) => {
                log::warn!("transient proxy accept error: {err}");
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue;
            }
        };
        let _ = stream.set_nodelay(true);
        let client = client.clone();
        let headers = headers.clone();
        let target_host = target_host.clone();
        let active_conns = Arc::clone(&active_connections);
        let activity = Arc::clone(&last_activity);
        let cache = proxy_cache.clone();

        active_conns.fetch_add(1, Ordering::Relaxed);
        {
            if let Ok(mut lock) = activity.lock() {
                *lock = Instant::now();
            }
        }

        let sub_opt = subtitle_url.clone();
        tokio::spawn(async move {
            let _guard = ConnectionGuard {
                conns: active_conns,
                activity,
            };
            let _ = handle_connection(
                stream,
                port,
                &client,
                &headers,
                target_host.as_deref(),
                sub_opt.as_deref(),
                max_height,
                &cache,
            )
            .await;
        });
    }
}

fn extract_host_authority(url: &str) -> Option<String> {
    let after_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let authority = after_scheme.split('/').next()?;
    if authority.is_empty() {
        None
    } else {
        Some(authority.to_string())
    }
}

const CHUNK_SIZE_95K: usize = 95 * 1024;
const MAX_CONCURRENT_CHUNKS: usize = 8;

async fn fetch_segment_concurrent_95k(
    client: &reqwest::Client,
    url: &str,
    headers: &[(String, String)],
) -> Result<bytes::Bytes, Box<dyn std::error::Error + Send + Sync>> {
    let mut req = client
        .get(url)
        .header("Range", format!("bytes=0-{}", CHUNK_SIZE_95K - 1));
    for (k, v) in headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let res = req.send().await?;
    let status = res.status();
    if !status.is_success() {
        return Err(format!("Upstream segment fetch error {status}").into());
    }

    let content_range = res
        .headers()
        .get("content-range")
        .and_then(|v| v.to_str().ok());
    let total_len = content_range.and_then(|cr| {
        cr.split('/').nth(1).and_then(|s| s.parse::<usize>().ok())
    });

    let first_chunk = res.bytes().await?;
    let Some(total_size) = total_len else {
        return Ok(first_chunk);
    };

    if total_size <= first_chunk.len() {
        return Ok(first_chunk);
    }

    let mut ranges = Vec::new();
    let mut start = first_chunk.len();
    while start < total_size {
        let end = (start + CHUNK_SIZE_95K - 1).min(total_size - 1);
        ranges.push((start, end));
        start = end + 1;
    }

    let fetched_chunks = futures::stream::iter(ranges.into_iter().enumerate())
        .map(|(idx, (s, e))| {
            let client = client.clone();
            let url = url.to_string();
            let headers = headers.to_vec();
            async move {
                let mut req = client.get(&url).header("Range", format!("bytes={s}-{e}"));
                for (k, v) in &headers {
                    req = req.header(k.as_str(), v.as_str());
                }
                let res = req.send().await?;
                let bytes = res.bytes().await?;
                Ok::<(usize, usize, bytes::Bytes), Box<dyn std::error::Error + Send + Sync>>((
                    idx, s, bytes,
                ))
            }
        })
        .buffer_unordered(MAX_CONCURRENT_CHUNKS)
        .collect::<Vec<_>>()
        .await;

    let mut all_chunks = Vec::with_capacity(fetched_chunks.len() + 1);
    all_chunks.push((0, first_chunk));
    for chunk_res in fetched_chunks {
        let (_idx, start_pos, chunk_bytes) = chunk_res?;
        all_chunks.push((start_pos, chunk_bytes));
    }
    all_chunks.sort_by_key(|(start_pos, _)| *start_pos);

    let mut full_body = bytes::BytesMut::with_capacity(total_size);
    for (_, chunk_bytes) in all_chunks {
        full_body.extend_from_slice(&chunk_bytes);
    }

    Ok(full_body.freeze())
}

pub fn get_next_segment_urls(url: &str, count: usize) -> Vec<String> {
    let (base_path, query_suffix) = match url.find(".m4s") {
        Some(pos) => (&url[..pos], &url[pos + 4..]),
        None => return Vec::new(),
    };

    let bytes = base_path.as_bytes();
    let mut end_digits = bytes.len();
    while end_digits > 0 && bytes[end_digits - 1].is_ascii_digit() {
        end_digits -= 1;
    }
    if end_digits == bytes.len() {
        return Vec::new();
    }

    let prefix = &base_path[..end_digits];
    let digits_str = &base_path[end_digits..];
    let pad_len = digits_str.len();
    let Ok(current_seq) = digits_str.parse::<u64>() else {
        return Vec::new();
    };

    let mut next_urls = Vec::with_capacity(count + 2);

    if prefix.contains("stream0-") {
        let audio_prefix = prefix.replace("stream0-", "stream1-");
        for step in 0..=count as u64 {
            let seq = current_seq + step;
            next_urls.push(format!("{audio_prefix}{seq:0pad_len$}.m4s{query_suffix}"));
        }
    }

    for step in 1..=count as u64 {
        let seq = current_seq + step;
        next_urls.push(format!("{prefix}{seq:0pad_len$}.m4s{query_suffix}"));
    }

    next_urls
}

fn spawn_prefetch_next_segments(
    client: reqwest::Client,
    auth_headers: Vec<(String, String)>,
    current_url: String,
    cache: ProxyCache,
) {
    tokio::spawn(async move {
        let next_urls = get_next_segment_urls(&current_url, 3);
        for next_url in next_urls {
            {
                let read = cache.segments.read().await;
                if read.get(&next_url).is_some() {
                    continue;
                }
            }
            if let Ok(data) =
                fetch_segment_concurrent_95k(&client, &next_url, &auth_headers).await
            {
                cache.segments.write().await.insert(next_url, data);
            }
        }
    });
}

async fn warmup_dash_session(
    client: &reqwest::Client,
    target_url: &str,
    headers: &[(String, String)],
    proxy_port: u16,
    target_host: Option<&str>,
    subtitle_url: Option<&str>,
    max_height: Option<u64>,
    cache: &ProxyCache,
) {
    if !target_url.ends_with(".mpd") && !target_url.contains("/dash/") {
        return;
    }
    let mut req = client.get(target_url);
    for (k, v) in headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let Ok(res) = req.send().await else {
        return;
    };
    if !res.status().is_success() {
        return;
    }
    let Ok(manifest_bytes) = res.bytes().await else {
        return;
    };
    let manifest_str = String::from_utf8_lossy(&manifest_bytes);
    let rewritten = rewrite_dash_manifest(
        &manifest_str,
        proxy_port,
        target_host,
        subtitle_url,
        max_height,
    );
    *cache.manifest.write().await = Some(rewritten.clone());

    let mut warm_urls = Vec::new();
    for line in rewritten.lines() {
        if line.contains(".m4s") {
            for token in line.split(['"', '\'', ' ', '<', '>']) {
                if token.contains(".m4s") {
                    if let Some(target) = extract_target_url(token) {
                        warm_urls.push(target);
                    } else if token.starts_with("http://") || token.starts_with("https://") {
                        warm_urls.push(token.to_string());
                    }
                }
            }
        }
        if warm_urls.len() >= 2 {
            break;
        }
    }

    for warm_url in warm_urls {
        if let Ok(data) = fetch_segment_concurrent_95k(client, &warm_url, headers).await {
            cache.segments.write().await.insert(warm_url, data);
        }
    }
}

pub fn parse_range(range_header: &str, total_len: usize) -> Option<(usize, usize)> {
    if total_len == 0 {
        return None;
    }
    let s = range_header.strip_prefix("bytes=")?;
    let (start_str, end_str) = s.split_once('-')?;
    let (start, end) = if start_str.is_empty() {
        let suffix = end_str.parse::<usize>().ok()?;
        let start = total_len.saturating_sub(suffix);
        let end = total_len.saturating_sub(1);
        (start, end)
    } else {
        let start = start_str.parse::<usize>().ok()?;
        let end = if end_str.is_empty() {
            total_len.saturating_sub(1)
        } else {
            end_str
                .parse::<usize>()
                .ok()?
                .min(total_len.saturating_sub(1))
        };
        (start, end)
    };
    if start <= end && start < total_len {
        Some((start, end))
    } else {
        None
    }
}

async fn handle_connection(
    stream: TcpStream,
    proxy_port: u16,
    client: &reqwest::Client,
    auth_headers: &[(String, String)],
    target_host: Option<&str>,
    subtitle_url: Option<&str>,
    max_height: Option<u64>,
    cache: &ProxyCache,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (reader, writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut writer = tokio::io::BufWriter::with_capacity(128 * 1024, writer);

    let mut request_line = String::new();
    let n = buf_reader.read_line(&mut request_line).await?;
    if n == 0 {
        return Ok(());
    }
    if request_line.len() > MAX_LINE_BYTES {
        writer
            .write_all(b"HTTP/1.1 431 Request Header Fields Too Large\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await?;
        writer.flush().await?;
        return Ok(());
    }

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path_and_query = parts.next().unwrap_or("/");

    let mut range_header = None;
    let mut header_count = 0usize;
    loop {
        let mut header_line = String::new();
        if buf_reader.read_line(&mut header_line).await? == 0 {
            break;
        }
        let trimmed = header_line.trim();
        if trimmed.is_empty() {
            break;
        }
        if header_line.len() > MAX_LINE_BYTES {
            break;
        }
        header_count += 1;
        if header_count > MAX_HEADERS {
            break;
        }
        if let Some((name, val)) = trimmed.split_once(':') {
            if name.trim().eq_ignore_ascii_case("range") {
                range_header = Some(val.trim().to_string());
            }
        }
    }

    let target_url = match extract_target_url(path_and_query) {
        Some(url) => url,
        None => {
            let response =
                "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            writer.write_all(response.as_bytes()).await?;
            writer.flush().await?;
            return Ok(());
        }
    };
    let extracted_host = extract_host_authority(&target_url);
    let sub_host = subtitle_url.and_then(extract_host_authority);
    let is_allowed = match (target_host, extracted_host.as_deref()) {
        (Some(allowed), Some(extracted)) => {
            extracted == allowed || (sub_host.is_some() && extracted_host == sub_host)
        }
        _ => false,
    };
    if !is_allowed {
        let response = "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        writer.write_all(response.as_bytes()).await?;
        writer.flush().await?;
        return Ok(());
    }

    let is_dash_manifest = target_url.ends_with(".mpd")
        || target_url.contains("/dash/")
            && (target_url.ends_with("/index") || target_url.ends_with("/manifest"));

    // Check RAM cached manifest first
    if is_dash_manifest && method == "GET" {
        if let Some(cached_manifest) = cache.manifest.read().await.as_ref() {
            let headers_out = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/dash+xml\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
                cached_manifest.len()
            );
            writer.write_all(headers_out.as_bytes()).await?;
            writer.write_all(cached_manifest.as_bytes()).await?;
            writer.flush().await?;
            return Ok(());
        }
    }

    // Check RAM cached segment
    let is_m4s_segment = target_url.contains(".m4s");
    if is_m4s_segment && method == "GET" {
        if let Some(cached_data) = cache.segments.read().await.get(&target_url) {
            if let Some(ref range) = range_header {
                if let Some((start, end)) = parse_range(range, cached_data.len()) {
                    let slice = &cached_data[start..=end];
                    let res_hdr = format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Type: video/iso.segment\r\nContent-Range: bytes {start}-{end}/{}\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
                        cached_data.len(),
                        slice.len()
                    );
                    writer.write_all(res_hdr.as_bytes()).await?;
                    writer.write_all(slice).await?;
                    writer.flush().await?;
                    spawn_prefetch_next_segments(
                        client.clone(),
                        auth_headers.to_vec(),
                        target_url.clone(),
                        cache.clone(),
                    );
                    return Ok(());
                }
            }

            let res_hdr = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: video/iso.segment\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
                cached_data.len()
            );
            writer.write_all(res_hdr.as_bytes()).await?;
            writer.write_all(&cached_data).await?;
            writer.flush().await?;
            spawn_prefetch_next_segments(
                client.clone(),
                auth_headers.to_vec(),
                target_url.clone(),
                cache.clone(),
            );
            return Ok(());
        }

        // Fetch segment with concurrent 95KB HTTP Range requests
        if let Ok(segment_data) =
            fetch_segment_concurrent_95k(client, &target_url, auth_headers).await
        {
            cache
                .segments
                .write()
                .await
                .insert(target_url.clone(), segment_data.clone());

            if let Some(ref range) = range_header {
                if let Some((start, end)) = parse_range(range, segment_data.len()) {
                    let slice = &segment_data[start..=end];
                    let res_hdr = format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Type: video/iso.segment\r\nContent-Range: bytes {start}-{end}/{}\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
                        segment_data.len(),
                        slice.len()
                    );
                    writer.write_all(res_hdr.as_bytes()).await?;
                    writer.write_all(slice).await?;
                    writer.flush().await?;
                    spawn_prefetch_next_segments(
                        client.clone(),
                        auth_headers.to_vec(),
                        target_url.clone(),
                        cache.clone(),
                    );
                    return Ok(());
                }
            }

            let res_hdr = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: video/iso.segment\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
                segment_data.len()
            );
            writer.write_all(res_hdr.as_bytes()).await?;
            writer.write_all(&segment_data).await?;
            writer.flush().await?;
            spawn_prefetch_next_segments(
                client.clone(),
                auth_headers.to_vec(),
                target_url.clone(),
                cache.clone(),
            );
            return Ok(());
        }
    }

    let mut req = match method {
        "HEAD" => client.head(&target_url),
        _ => client.get(&target_url),
    };

    let forward_all_headers = extracted_host.as_deref() == target_host;
    for (name, val) in auth_headers {
        if forward_all_headers || name.eq_ignore_ascii_case("user-agent") {
            req = req.header(name.as_str(), val.as_str());
        }
    }
    if let Some(range) = range_header {
        req = req.header("Range", range);
    }

    let upstream_res = match req.send().await {
        Ok(res) => res,
        Err(e) => {
            let body = format!("Gateway Error: {e}");
            let response = format!(
                "HTTP/1.1 502 Bad Gateway\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            writer.write_all(response.as_bytes()).await?;
            writer.flush().await?;
            return Ok(());
        }
    };

    let status = upstream_res.status();

    let content_length = upstream_res
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok());

    let is_dash = is_dash_manifest
        || upstream_res
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|ct| ct.contains("dash+xml") || ct.contains("xml"))
            .unwrap_or(false);

    let within_manifest_limit = content_length.is_none_or(|len| len <= MAX_MANIFEST_BYTES);

    if is_dash && status.is_success() && within_manifest_limit {
        let manifest_bytes = upstream_res.bytes().await?;
        if manifest_bytes.len() > MAX_MANIFEST_BYTES {
            let body = "Manifest too large";
            writer
                .write_all(
                    format!(
                        "HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await?;
            writer.flush().await?;
            return Ok(());
        }
        let manifest_str = String::from_utf8_lossy(&manifest_bytes);
        let rewritten = rewrite_dash_manifest(
            &manifest_str,
            proxy_port,
            target_host,
            subtitle_url,
            max_height,
        );
        *cache.manifest.write().await = Some(rewritten.clone());
        let rewritten_bytes = rewritten.as_bytes();

        let headers_out = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/dash+xml\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
            rewritten_bytes.len()
        );
        writer.write_all(headers_out.as_bytes()).await?;
        writer.write_all(rewritten_bytes).await?;
        writer.flush().await?;
        return Ok(());
    }

    let status_line = format!(
        "HTTP/1.1 {} {}\r\n",
        status.as_u16(),
        status.canonical_reason().unwrap_or("OK")
    );
    writer.write_all(status_line.as_bytes()).await?;

    let headers_bytes = format_proxy_response_headers(upstream_res.headers(), &target_url);
    writer.write_all(&headers_bytes).await?;
    writer.flush().await?;

    let mut stream = upstream_res.bytes_stream();
    loop {
        let chunk_result =
            tokio::time::timeout(Duration::from_secs(CHUNK_IDLE_TIMEOUT_SECS), stream.next()).await;
        match chunk_result {
            Ok(Some(Ok(chunk))) => {
                writer.write_all(&chunk).await?;
                writer.flush().await?;
            }
            Ok(Some(Err(e))) => return Err(Box::new(e)),
            Ok(None) => break,
            Err(_elapsed) => break,
        }
    }
    writer.flush().await?;

    Ok(())
}

fn format_proxy_response_headers(
    headers: &reqwest::header::HeaderMap,
    target_url: &str,
) -> Vec<u8> {
    let mut out = Vec::new();
    let clean_path = target_url
        .split('?')
        .next()
        .unwrap_or("")
        .split('#')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let is_srt = clean_path.ends_with(".srt");
    let is_vtt = clean_path.ends_with(".vtt");

    for (header_name, header_val) in headers {
        let name_str = header_name.as_str();
        if (is_srt || is_vtt) && name_str.eq_ignore_ascii_case("content-type") {
            continue;
        }
        if name_str.eq_ignore_ascii_case("content-type")
            || name_str.eq_ignore_ascii_case("content-length")
            || name_str.eq_ignore_ascii_case("content-range")
            || name_str.eq_ignore_ascii_case("accept-ranges")
        {
            if let Ok(val_str) = header_val.to_str() {
                out.extend_from_slice(format!("{name_str}: {val_str}\r\n").as_bytes());
            }
        }
    }

    out.extend_from_slice(b"Access-Control-Allow-Origin: *\r\n");
    if is_srt {
        out.extend_from_slice(b"Content-Type: application/x-subrip\r\n");
    } else if is_vtt {
        out.extend_from_slice(b"Content-Type: text/vtt\r\n");
    }
    out.extend_from_slice(b"Connection: close\r\n\r\n");
    out
}

fn extract_target_url(path_and_query: &str) -> Option<String> {
    let raw = path_and_query.strip_prefix('/')?;
    if let Some(rest) = raw.strip_prefix("sub/") {
        if let Ok(decoded) = percent_encoding::percent_decode_str(rest).decode_utf8() {
            let s = decoded.into_owned();
            if s.starts_with("http://") || s.starts_with("https://") {
                return Some(s);
            }
        }
    }
    if let Some(rest) = raw.strip_prefix("https/") {
        Some(format!("https://{rest}"))
    } else if let Some(rest) = raw.strip_prefix("http/") {
        Some(format!("http://{rest}"))
    } else if raw.starts_with("proxy?") || raw.contains("&url=") || raw.starts_with("proxy?url=") {
        let query_start = raw.find('?')?;
        let query = &raw[query_start + 1..];
        for pair in query.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                if k == "url" {
                    return percent_encoding::percent_decode_str(v)
                        .decode_utf8()
                        .ok()
                        .map(|s| s.into_owned());
                }
            }
        }
        None
    } else {
        None
    }
}


fn rewrite_dash_manifest(
    manifest: &str,
    proxy_port: u16,
    target_host: Option<&str>,
    subtitle_url: Option<&str>,
    max_height: Option<u64>,
) -> String {
    let Some(host) = target_host else {
        return manifest.to_string();
    };

    let https_prefix = format!("https://{host}/");
    let http_prefix = format!("http://{host}/");

    let proxy_https = format!("http://127.0.0.1:{proxy_port}/https/{host}/");
    let proxy_http = format!("http://127.0.0.1:{proxy_port}/http/{host}/");

    let mut rewritten = manifest
        .replace(&https_prefix, &proxy_https)
        .replace(&http_prefix, &proxy_http);

    if let Some(sub) = subtitle_url {
        if !sub.is_empty() {
            let encoded_sub =
                percent_encoding::utf8_percent_encode(sub, percent_encoding::NON_ALPHANUMERIC);
            let sub_proxy_url = format!("http://127.0.0.1:{proxy_port}/sub/{encoded_sub}");
            let sub_adaptation_set = format!(
                r#"<AdaptationSet contentType="text" mimeType="text/vtt" lang="en">
    <Role schemeIdUri="urn:mpeg:dash:role:2011" value="subtitle"/>
    <Representation id="sub_en" bandwidth="1000">
      <BaseURL>{sub_proxy_url}</BaseURL>
    </Representation>
  </AdaptationSet>
</Period>"#
            );
            if rewritten.contains("</Period>") {
                rewritten = rewritten.replacen("</Period>", &sub_adaptation_set, 1);
            }
        }
    }

    if let Some(max_h) = max_height {
        rewritten = prune_dash_manifest_max_height(&rewritten, max_h);
    }

    rewritten
}

pub fn prune_dash_manifest_max_height(manifest: &str, max_height: u64) -> String {
    let mut out = String::with_capacity(manifest.len());
    let mut search_idx = 0;

    while let Some(adapt_start) = manifest[search_idx..].find("<AdaptationSet") {
        let abs_adapt_start = search_idx + adapt_start;
        out.push_str(&manifest[search_idx..abs_adapt_start]);

        let Some(adapt_end_rel) = manifest[abs_adapt_start..].find("</AdaptationSet>") else {
            out.push_str(&manifest[abs_adapt_start..]);
            search_idx = manifest.len();
            break;
        };
        let abs_adapt_end = abs_adapt_start + adapt_end_rel + "</AdaptationSet>".len();
        let adapt_block = &manifest[abs_adapt_start..abs_adapt_end];

        let pruned_block = prune_adaptation_set_representations(adapt_block, max_height);
        out.push_str(&pruned_block);
        search_idx = abs_adapt_end;
    }

    out.push_str(&manifest[search_idx..]);
    out
}

fn prune_adaptation_set_representations(adapt_block: &str, max_height: u64) -> String {
    struct RepBlock {
        start: usize,
        end: usize,
        height: Option<u64>,
    }

    let mut reps = Vec::new();
    let mut cur = 0;

    while let Some(rel_start) = adapt_block[cur..].find("<Representation") {
        let rep_start = cur + rel_start;
        let Some(rel_close) = adapt_block[rep_start..].find('>') else {
            break;
        };
        let tag_close = rep_start + rel_close;
        let is_self_closing = adapt_block[rep_start..tag_close].trim_end().ends_with('/');

        let rep_end = if is_self_closing {
            tag_close + 1
        } else if let Some(rel_end) = adapt_block[tag_close..].find("</Representation>") {
            tag_close + rel_end + "</Representation>".len()
        } else {
            tag_close + 1
        };

        let tag_header = &adapt_block[rep_start..=tag_close];
        let height = extract_xml_attr_u64(tag_header, "height");

        reps.push(RepBlock {
            start: rep_start,
            end: rep_end,
            height,
        });

        cur = rep_end;
    }

    let video_reps: Vec<&RepBlock> = reps.iter().filter(|r| r.height.is_some()).collect();
    if video_reps.is_empty() {
        return adapt_block.to_string();
    }

    let any_within_limit = video_reps.iter().any(|r| r.height.unwrap() <= max_height);
    let to_remove: std::collections::HashSet<usize> = if any_within_limit {
        reps.iter()
            .enumerate()
            .filter(|(_, r)| r.height.map(|h| h > max_height).unwrap_or(false))
            .map(|(idx, _)| idx)
            .collect()
    } else {
        let min_h = video_reps.iter().map(|r| r.height.unwrap()).min().unwrap();
        reps.iter()
            .enumerate()
            .filter(|(_, r)| r.height.map(|h| h > min_h).unwrap_or(false))
            .map(|(idx, _)| idx)
            .collect()
    };

    if to_remove.is_empty() {
        return adapt_block.to_string();
    }

    let mut result = String::with_capacity(adapt_block.len());
    let mut last_idx = 0;
    for (idx, r) in reps.iter().enumerate() {
        if to_remove.contains(&idx) {
            result.push_str(&adapt_block[last_idx..r.start]);
            last_idx = r.end;
        }
    }
    result.push_str(&adapt_block[last_idx..]);
    result
}

fn extract_xml_attr_u64(tag: &str, attr: &str) -> Option<u64> {
    for quote in ['"', '\''] {
        let needle = format!("{attr}={quote}");
        if let Some(pos) = tag.find(&needle) {
            let rest = &tag[pos + needle.len()..];
            if let Some(end_pos) = rest.find(quote) {
                if let Ok(val) = rest[..end_pos].trim().parse::<u64>() {
                    return Some(val);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_target_url() {
        assert_eq!(
            extract_target_url("/https/sacdn.example.com/dash/index.mpd").as_deref(),
            Some("https://sacdn.example.com/dash/index.mpd")
        );
        assert_eq!(
            extract_target_url("/http/example.com/video.mp4").as_deref(),
            Some("http://example.com/video.mp4")
        );
        assert_eq!(
            extract_target_url("/https/example.com/seg.m4s?token=abc&exp=123").as_deref(),
            Some("https://example.com/seg.m4s?token=abc&exp=123")
        );
        assert_eq!(
            extract_target_url("/proxy?url=https%3A%2F%2Fexample.com%2Ffallback.mpd").as_deref(),
            Some("https://example.com/fallback.mpd")
        );
        assert_eq!(extract_target_url("/invalid/path"), None);
    }
    #[test]
    fn test_extract_target_url_sub_rejects_non_http() {
        assert_eq!(
            extract_target_url("/sub/file%3A%2F%2F%2Fetc%2Fpasswd"),
            None
        );
        assert_eq!(extract_target_url("/sub/data%3Atext%2Fhtml%2Chello"), None);
        assert!(
            extract_target_url("/sub/https%3A%2F%2Fcdn.example.com%2Fsub.vtt")
                .as_deref()
                .unwrap_or("")
                .starts_with("https://")
        );
    }

    #[test]
    fn test_extract_host_authority_strips_path_preserves_port() {
        assert_eq!(
            extract_host_authority("https://cdn.example.com/dash/index.mpd").as_deref(),
            Some("cdn.example.com")
        );
        assert_eq!(
            extract_host_authority("http://cdn.example.com:8080/dash/index.mpd").as_deref(),
            Some("cdn.example.com:8080")
        );
        assert_eq!(extract_host_authority("not-a-url"), None);
    }
    #[test]
    fn test_host_whitelist_allows_target_and_subtitle_hosts() {
        let target_host = "video.example.com";
        let subtitle_url = "https://captions.example.com/sub.srt";
        let sub_host = extract_host_authority(subtitle_url);

        let is_allowed = |url: &str| -> bool {
            let host = extract_host_authority(url);
            host.as_deref() == Some(target_host) || (sub_host.is_some() && host == sub_host)
        };

        assert!(is_allowed("https://video.example.com/chunk.m4s"));
        assert!(is_allowed("https://captions.example.com/sub.srt"));
        assert!(!is_allowed("https://evil.example.com/steal"));
        assert!(!is_allowed("https://sub.evil.com/fake"));
    }
    #[test]
    fn test_host_whitelist_rejects_missing_target_host() {
        let target_host: Option<&str> = None;
        let subtitle_url: Option<&str> = None;
        let sub_host = subtitle_url.and_then(extract_host_authority);

        let is_allowed = |url: &str| -> bool {
            let extracted_host = extract_host_authority(url);
            match (target_host, extracted_host.as_deref()) {
                (Some(allowed), Some(extracted)) => {
                    extracted == allowed || (sub_host.is_some() && extracted_host == sub_host)
                }
                _ => false,
            }
        };

        assert!(!is_allowed("https://video.example.com/chunk.m4s"));
        assert!(!is_allowed("http://127.0.0.1:8080"));
    }

    #[test]
    fn test_rewrite_dash_manifest_scoped_to_target_host() {
        let manifest = r#"<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" xsi:schemaLocation="urn:mpeg:dash:schema:mpd:2011 https://standards.iso.org/schema.xsd">
<Period>
  <AdaptationSet>
    <Representation id="1080p">
      <BaseURL>https://sacdn.example.com/dash/123/1080.mp4</BaseURL>
      <SegmentTemplate initialization="https://sacdn.example.com/dash/123/init.m4s" media="seg_$Number$.m4s" />
    </Representation>
  </AdaptationSet>
</Period>
</MPD>"#;

        let rewritten = rewrite_dash_manifest(manifest, 8888, Some("sacdn.example.com"), None, None);

        assert!(
            rewritten.contains("https://standards.iso.org/schema.xsd"),
            "XML namespace schema must not be corrupted"
        );

        assert!(
            rewritten.contains("http://127.0.0.1:8888/https/sacdn.example.com/dash/123/1080.mp4"),
            "Matching host BaseURL must be rewritten to proxy route"
        );

        assert!(
            rewritten.contains("http://127.0.0.1:8888/https/sacdn.example.com/dash/123/init.m4s"),
            "Matching host initialization URL must be rewritten to proxy route"
        );

        assert!(
            rewritten.contains("media=\"seg_$Number$.m4s\""),
            "Relative media template must remain relative"
        );
    }

    #[test]
    fn test_rewrite_dash_manifest_with_explicit_port() {
        let manifest = r#"<MPD><Period><BaseURL>https://cdn.example.com:8080/dash/seg.mp4</BaseURL></Period></MPD>"#;
        let rewritten = rewrite_dash_manifest(manifest, 9999, Some("cdn.example.com:8080"), None, None);
        assert!(
            rewritten.contains("http://127.0.0.1:9999/https/cdn.example.com:8080/dash/seg.mp4"),
            "Port must be preserved in proxy route"
        );
    }
    #[test]
    fn test_format_proxy_response_headers_for_srt_with_query_and_case() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("content-type", "text/plain".parse().unwrap());
        headers.insert("content-length", "1234".parse().unwrap());
        headers.insert("accept-ranges", "bytes".parse().unwrap());
        headers.insert("server", "cloudflare".parse().unwrap());

        let url = "https://cdn.example.com/subs.SRT?token=abc123&expires=999#top";
        let out = String::from_utf8(format_proxy_response_headers(&headers, url)).unwrap();

        assert_eq!(out.matches("Access-Control-Allow-Origin: *").count(), 1);
        assert_eq!(out.matches("Content-Type: application/x-subrip").count(), 1);
        assert_eq!(out.matches("Connection: close").count(), 1);
        assert!(!out.contains("text/plain"));
        assert!(out.contains("content-length: 1234"));
        assert!(out.contains("accept-ranges: bytes"));
    }

    #[test]
    fn test_format_proxy_response_headers_for_vtt() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("content-type", "application/octet-stream".parse().unwrap());
        headers.insert("content-length", "500".parse().unwrap());

        let url = "https://cdn.example.com/subs.vtt";
        let out = String::from_utf8(format_proxy_response_headers(&headers, url)).unwrap();

        assert_eq!(out.matches("Access-Control-Allow-Origin: *").count(), 1);
        assert_eq!(out.matches("Content-Type: text/vtt").count(), 1);
        assert!(!out.contains("application/octet-stream"));
        assert!(out.contains("content-length: 500"));
    }

    #[test]
    fn test_format_proxy_response_headers_for_media_stream() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("content-type", "video/mp4".parse().unwrap());
        headers.insert("content-length", "10000000".parse().unwrap());

        let url = "https://cdn.example.com/video.mp4";
        let out = String::from_utf8(format_proxy_response_headers(&headers, url)).unwrap();

        assert_eq!(out.matches("Access-Control-Allow-Origin: *").count(), 1);
        assert!(out.contains("content-type: video/mp4"));
        assert!(!out.contains("application/x-subrip"));
        assert!(!out.contains("text/vtt"));
    }

    #[test]
    fn test_prune_dash_manifest_max_height() {
        let manifest = r#"<MPD>
<Period>
  <AdaptationSet id="video" mimeType="video/mp4">
    <Representation id="1080p" height="1080" bandwidth="5000000">
      <BaseURL>1080.mp4</BaseURL>
    </Representation>
    <Representation id="720p" height="720" bandwidth="2500000">
      <BaseURL>720.mp4</BaseURL>
    </Representation>
    <Representation id="480p" height="480" bandwidth="1000000" />
  </AdaptationSet>
  <AdaptationSet id="audio" mimeType="audio/mp4">
    <Representation id="audio_en" bandwidth="128000">
      <BaseURL>audio.mp4</BaseURL>
    </Representation>
  </AdaptationSet>
</Period>
</MPD>"#;

        let pruned = prune_dash_manifest_max_height(manifest, 720);
        assert!(!pruned.contains("1080p"), "1080p must be pruned");
        assert!(pruned.contains("720p"), "720p must be retained");
        assert!(pruned.contains("480p"), "480p must be retained");
        assert!(pruned.contains("audio_en"), "Audio must be retained untouched");
    }

    #[test]
    fn test_prune_dash_manifest_max_height_fallback_to_lowest() {
        let manifest = r#"<MPD>
<Period>
  <AdaptationSet id="video">
    <Representation id="4k" height="2160">
      <BaseURL>4k.mp4</BaseURL>
    </Representation>
    <Representation id="1080p" height="1080">
      <BaseURL>1080.mp4</BaseURL>
    </Representation>
  </AdaptationSet>
</Period>
</MPD>"#;

        // When max_height is 720, but only 2160 and 1080 exist, don't drop all video streams
        let pruned = prune_dash_manifest_max_height(manifest, 720);
        assert!(!pruned.contains("4k"), "Higher 4k stream should be pruned");
        assert!(pruned.contains("1080p"), "Lowest available video stream must be preserved as fallback");
    }

    #[test]
    fn test_get_next_segment_urls() {
        let url = "https://example.com/dash/123/chunk-stream0-00005.m4s?token=abc";
        let next = get_next_segment_urls(url, 3);
        assert!(next.contains(&"https://example.com/dash/123/chunk-stream0-00006.m4s?token=abc".to_string()));
        assert!(next.contains(&"https://example.com/dash/123/chunk-stream0-00007.m4s?token=abc".to_string()));
        assert!(next.contains(&"https://example.com/dash/123/chunk-stream0-00008.m4s?token=abc".to_string()));
        assert!(next.contains(&"https://example.com/dash/123/chunk-stream1-00005.m4s?token=abc".to_string()));
    }

    #[test]
    fn test_segment_cache_bounded() {
        let mut cache = SegmentCache::new(3);
        cache.insert("s1".to_string(), bytes::Bytes::from_static(b"data1"));
        cache.insert("s2".to_string(), bytes::Bytes::from_static(b"data2"));
        cache.insert("s3".to_string(), bytes::Bytes::from_static(b"data3"));
        assert_eq!(cache.get("s1").as_deref(), Some(&b"data1"[..]));

        // Inserting 4th item evicts oldest (s1)
        cache.insert("s4".to_string(), bytes::Bytes::from_static(b"data4"));
        assert_eq!(cache.get("s1"), None);
        assert_eq!(cache.get("s2").as_deref(), Some(&b"data2"[..]));
        assert_eq!(cache.get("s4").as_deref(), Some(&b"data4"[..]));
    }

    #[test]
    fn test_parse_range() {
        assert_eq!(parse_range("bytes=0-99", 1000), Some((0, 99)));
        assert_eq!(parse_range("bytes=100-", 1000), Some((100, 999)));
        assert_eq!(parse_range("bytes=-200", 1000), Some((800, 999)));
        assert_eq!(parse_range("bytes=1500-1600", 1000), None);
        assert_eq!(parse_range("invalid", 1000), None);
    }
}
