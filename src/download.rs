use reqwest::{
    Client, StatusCode,
    header::{ACCEPT_RANGES, CONTENT_RANGE, ETAG, IF_RANGE, LAST_MODIFIED, RANGE},
};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use std::io::SeekFrom;
use thiserror::Error;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

const MAX_ATTEMPTS: usize = 4;
const SEGMENT_THRESHOLD: u64 = 32 * 1024 * 1024;
const MAX_SEGMENTS: usize = 8;
pub const DEFAULT_STREAM_NAME: &str = "SumanMovies-Tui_Stream";

pub fn safe_file_stem(value: &str) -> String {
    let mut stem = value
        .chars()
        .take(120)
        .map(|character| {
            if character.is_control()
                || matches!(
                    character,
                    '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
                )
            {
                '_'
            } else {
                character
            }
        })
        .collect::<String>();
    stem = stem.trim_matches(['.', ' ', '_']).to_string();
    if stem.is_empty() {
        return DEFAULT_STREAM_NAME.into();
    }
    let upper = stem.to_ascii_uppercase();
    let reserved = matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || upper
            .strip_prefix("COM")
            .or_else(|| upper.strip_prefix("LPT"))
            .is_some_and(|number| {
                number.len() == 1 && number.bytes().all(|byte| matches!(byte, b'1'..=b'9'))
            });
    if reserved {
        stem.push('_');
    }
    stem
}

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub bytes_per_second: f64,
    pub attempt: usize,
    pub workers: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadOutcome {
    Completed { bytes: u64 },
    Paused { bytes: u64 },
}

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("server returned HTTP {0}")]
    Http(StatusCode),
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("file error: {0}")]
    File(#[from] std::io::Error),
    #[error("invalid partial response: {0}")]
    InvalidRange(String),
    #[error("download ended at {downloaded} of {expected} bytes")]
    Incomplete { downloaded: u64, expected: u64 },
    #[error("download paused")]
    Paused,
}

impl DownloadError {
    pub fn user_message(&self) -> String {
        match self {
            Self::Http(status) => match *status {
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                    format!("Server refused download (HTTP {}).", status.as_u16())
                }
                StatusCode::NOT_FOUND | StatusCode::GONE => {
                    "File is no longer available on server.".to_string()
                }
                StatusCode::TOO_MANY_REQUESTS => {
                    "Server rate limit exceeded. Try again later.".to_string()
                }
                other if other.is_server_error() => {
                    format!("Server error (HTTP {}). Try again later.", other.as_u16())
                }
                other => format!("Server returned HTTP {}.", other.as_u16()),
            },
            Self::Network(error) => {
                if error.is_timeout() {
                    "Connection timed out.".to_string()
                } else if error.is_connect() {
                    "Cannot reach download server.".to_string()
                } else {
                    "Connection to server was lost.".to_string()
                }
            }
            Self::File(error) => format!("File write error: {}.", error.kind()),
            Self::InvalidRange(_) => "Server returned invalid partial response.".to_string(),
            Self::Incomplete {
                downloaded,
                expected,
            } => format!(
                "Download stopped at {:.1} of {:.1} MB.",
                *downloaded as f64 / 1_048_576.0,
                *expected as f64 / 1_048_576.0
            ),
            Self::Paused => "Download paused.".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ResumeMetadata {
    etag: Option<String>,
    last_modified: Option<String>,
    total: Option<u64>,
    segments: Option<usize>,
    segment_progress: Option<Vec<u64>>,
}

pub async fn download<F>(
    client: &Client,
    url: &str,
    destination: &Path,
    cancel: Arc<AtomicBool>,
    mut report: F,
) -> Result<DownloadOutcome, DownloadError>
where
    F: FnMut(DownloadProgress),
{
    let partial = sidecar_path(destination, "part");
    let metadata_path = sidecar_path(destination, "part.json");
    let mut metadata = read_metadata(&metadata_path).await;
    let started = Instant::now();
    let mut last_report = Instant::now() - Duration::from_secs(1);
    let mut last_error = None;
    let mut segmented_disabled = false;

    for attempt in 1..=MAX_ATTEMPTS {
        if cancel.load(Ordering::Relaxed) {
            return Ok(DownloadOutcome::Paused {
                bytes: file_len(&partial).await,
            });
        }

        let mut offset = file_len(&partial).await;
        let mut request = client.get(url);
        if offset > 0 {
            request = request.header(RANGE, format!("bytes={offset}-"));
            if let Some(validator) = metadata.etag.as_ref().or(metadata.last_modified.as_ref()) {
                request = request.header(IF_RANGE, validator);
            }
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                last_error = Some(DownloadError::Network(error));
                retry_delay(attempt).await;
                continue;
            }
        };

        if response.status() == StatusCode::RANGE_NOT_SATISFIABLE && metadata.total == Some(offset)
        {
            finalize(&partial, &metadata_path, destination).await?;
            return Ok(DownloadOutcome::Completed { bytes: offset });
        }
        if !response.status().is_success() {
            last_error = Some(DownloadError::Http(response.status()));
            retry_delay(attempt).await;
            continue;
        }

        if !segmented_disabled
            && offset == 0
            && response.status() == StatusCode::OK
            && response
                .headers()
                .get(ACCEPT_RANGES)
                .and_then(|value| value.to_str().ok())
                .is_none_or(|value| !value.eq_ignore_ascii_case("none"))
            && response
                .content_length()
                .is_some_and(|total| total >= SEGMENT_THRESHOLD)
        {
            let total = response.content_length().unwrap_or_default();
            let segments = segment_count(total);
            let mut current_metadata = ResumeMetadata {
                etag: header_string(&response, ETAG),
                last_modified: header_string(&response, LAST_MODIFIED),
                total: Some(total),
                segments: Some(segments),
                segment_progress: None,
            };
            if !metadata_matches(&metadata, &current_metadata) {
                remove_segment_files(destination).await;
            } else {
                current_metadata.segment_progress = metadata.segment_progress;
            }
            write_metadata(&metadata_path, &current_metadata).await?;
            drop(response);
            match download_segmented(
                client,
                url,
                destination,
                &metadata_path,
                current_metadata,
                cancel.clone(),
                &mut report,
            )
            .await
            {
                Err(DownloadError::InvalidRange(_)) => {
                    remove_segment_files(destination).await;
                    metadata = ResumeMetadata::default();
                    write_metadata(&metadata_path, &metadata).await?;
                    segmented_disabled = true;
                    continue;
                }
                result => return result,
            }
        }

        let response_total = if response.status() == StatusCode::PARTIAL_CONTENT {
            let content_range = response
                .headers()
                .get(CONTENT_RANGE)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| DownloadError::InvalidRange("Content-Range missing".into()))?;
            let (start, total) = parse_content_range(content_range)?;
            if start != offset {
                return Err(DownloadError::InvalidRange(format!(
                    "requested byte {offset}, received {start}"
                )));
            }
            total
        } else {
            if offset > 0 {
                truncate(&partial).await?;
                offset = 0;
            }
            response.content_length()
        };

        if offset > 0
            && let (Some(previous), Some(current)) = (metadata.total, response_total)
            && previous != current
        {
            truncate(&partial).await?;
            metadata = ResumeMetadata::default();
            write_metadata(&metadata_path, &metadata).await?;
            last_error = Some(DownloadError::InvalidRange(
                "remote file size changed; partial reset".into(),
            ));
            retry_delay(attempt).await;
            continue;
        }

        metadata.etag = header_string(&response, ETAG).or(metadata.etag);
        metadata.last_modified = header_string(&response, LAST_MODIFIED).or(metadata.last_modified);
        metadata.total = response_total.or(metadata.total);
        metadata.segments = None;
        metadata.segment_progress = None;
        write_metadata(&metadata_path, &metadata).await?;

        let raw_file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&partial)
            .await?;
        let mut file = tokio::io::BufWriter::with_capacity(256 * 1024, raw_file);
        let mut response = response;
        let mut downloaded = offset;
        let transfer_started = Instant::now();

        loop {
            if cancel.load(Ordering::Relaxed) {
                file.flush().await?;
                return Ok(DownloadOutcome::Paused { bytes: downloaded });
            }

            match tokio::time::timeout(Duration::from_secs(30), response.chunk()).await {
                Ok(Ok(Some(chunk))) => {
                    file.write_all(&chunk).await?;
                    downloaded += chunk.len() as u64;
                    if last_report.elapsed() >= Duration::from_millis(200) {
                        let elapsed = started.elapsed().as_secs_f64();
                        report(DownloadProgress {
                            downloaded,
                            total: metadata.total,
                            bytes_per_second: if elapsed > 0.0 {
                                downloaded.saturating_sub(offset) as f64 / elapsed
                            } else {
                                0.0
                            },
                            attempt,
                            workers: 1,
                        });
                        last_report = Instant::now();
                    }
                }
                Ok(Ok(None)) => {
                    file.flush().await?;
                    file.get_mut().sync_data().await?;
                    if let Some(expected) = metadata.total
                        && downloaded != expected
                    {
                        last_error = Some(DownloadError::Incomplete {
                            downloaded,
                            expected,
                        });
                        break;
                    }
                    report(DownloadProgress {
                        downloaded,
                        total: metadata.total.or(Some(downloaded)),
                        bytes_per_second: if transfer_started.elapsed().as_secs_f64() > 0.0 {
                            downloaded.saturating_sub(offset) as f64
                                / transfer_started.elapsed().as_secs_f64()
                        } else {
                            0.0
                        },
                        attempt,
                        workers: 1,
                    });
                    finalize(&partial, &metadata_path, destination).await?;
                    return Ok(DownloadOutcome::Completed { bytes: downloaded });
                }
                Ok(Err(error)) => {
                    file.flush().await?;
                    last_error = Some(DownloadError::Network(error));
                    break;
                }
                Err(_) => {
                    file.flush().await?;
                    last_error = Some(DownloadError::InvalidRange("read timeout".into()));
                    break;
                }
            }
        }

        retry_delay(attempt).await;
    }

    Err(last_error.unwrap_or(DownloadError::InvalidRange(
        "download failed without a response".into(),
    )))
}

async fn download_segmented<F>(
    client: &Client,
    url: &str,
    destination: &Path,
    metadata_path: &Path,
    metadata: ResumeMetadata,
    cancel: Arc<AtomicBool>,
    report: &mut F,
) -> Result<DownloadOutcome, DownloadError>
where
    F: FnMut(DownloadProgress),
{
    let total = metadata
        .total
        .ok_or_else(|| DownloadError::InvalidRange("segment total missing".into()))?;
    let segments = metadata.segments.unwrap_or_else(|| segment_count(total));
    let ranges = segment_ranges(total, segments);

    let partial = sidecar_path(destination, "part");

    // Pre-allocate the .part file to the exact total size
    let raw_file = tokio::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&partial)
        .await?;
    raw_file.set_len(total).await?;
    drop(raw_file);

    let initial_progress: Vec<u64> = if let Some(ref saved) = metadata.segment_progress {
        if saved.len() == segments {
            saved.clone()
        } else {
            vec![0u64; segments]
        }
    } else {
        vec![0u64; segments]
    };
    let initial: u64 = initial_progress.iter().sum();
    let downloaded = Arc::new(AtomicU64::new(initial));
    let trackers = Arc::new(tokio::sync::RwLock::new(initial_progress.clone()));

    let (progress_sender, mut progress_receiver) = tokio::sync::mpsc::channel::<(u64, usize)>(64);
    let mut tasks = tokio::task::JoinSet::new();
    let validator = metadata.etag.clone().or(metadata.last_modified.clone());

    for (index, (start, end)) in ranges.iter().copied().enumerate() {
        let client = client.clone();
        let url = url.to_string();
        let partial_path = partial.clone();
        let cancel = cancel.clone();
        let progress_sender = progress_sender.clone();
        let validator = validator.clone();
        let existing = initial_progress[index];
        let trackers_clone = Arc::clone(&trackers);
        tasks.spawn(async move {
            download_segment(
                &client,
                &url,
                &partial_path,
                start,
                end,
                total,
                existing,
                validator,
                cancel,
                progress_sender,
                trackers_clone,
                index,
            )
            .await
        });
    }
    drop(progress_sender);

    let started = Instant::now();
    let mut last_report = Instant::now() - Duration::from_secs(1);
    let mut finished = 0;
    while finished < segments {
        tokio::select! {
            progress = progress_receiver.recv() => {
                if let Some((bytes, attempt)) = progress {
                    let current = downloaded.fetch_add(bytes, Ordering::Relaxed) + bytes;
                    if last_report.elapsed() >= Duration::from_millis(200) {
                        let elapsed = started.elapsed().as_secs_f64();
                        report(DownloadProgress {
                            downloaded: current,
                            total: Some(total),
                            bytes_per_second: if elapsed > 0.0 {
                                current.saturating_sub(initial) as f64 / elapsed
                            } else {
                                0.0
                            },
                            attempt,
                            workers: segments,
                        });
                        last_report = Instant::now();
                    }
                }
            }
            result = tasks.join_next() => {
                match result {
                    Some(Ok(Ok(()))) => finished += 1,
                    Some(Ok(Err(DownloadError::Paused))) => {
                        tasks.abort_all();
                        let mut paused_meta = metadata;
                        paused_meta.segment_progress = Some(trackers.read().await.clone());
                        let _ = write_metadata(metadata_path, &paused_meta).await;
                        return Ok(DownloadOutcome::Paused {
                            bytes: downloaded.load(Ordering::Relaxed),
                        });
                    }
                    Some(Ok(Err(error))) => {
                        tasks.abort_all();
                        let mut paused_meta = metadata;
                        paused_meta.segment_progress = Some(trackers.read().await.clone());
                        let _ = write_metadata(metadata_path, &paused_meta).await;
                        return Err(error);
                    }
                    Some(Err(error)) => {
                        tasks.abort_all();
                        let mut paused_meta = metadata;
                        paused_meta.segment_progress = Some(trackers.read().await.clone());
                        let _ = write_metadata(metadata_path, &paused_meta).await;
                        return Err(DownloadError::InvalidRange(format!(
                            "download worker stopped: {error}"
                        )));
                    }
                    None => break,
                }
            }
        }
    }

    if cancel.load(Ordering::Relaxed) {
        let mut paused_meta = metadata;
        paused_meta.segment_progress = Some(trackers.read().await.clone());
        let _ = write_metadata(metadata_path, &paused_meta).await;
        return Ok(DownloadOutcome::Paused {
            bytes: downloaded.load(Ordering::Relaxed),
        });
    }

    if file_len(&partial).await != total {
        return Err(DownloadError::Incomplete {
            downloaded: file_len(&partial).await,
            expected: total,
        });
    }

    finalize(&partial, metadata_path, destination).await?;

    report(DownloadProgress {
        downloaded: total,
        total: Some(total),
        bytes_per_second: if started.elapsed().as_secs_f64() > 0.0 {
            total.saturating_sub(initial) as f64 / started.elapsed().as_secs_f64()
        } else {
            0.0
        },
        attempt: 1,
        workers: segments,
    });
    Ok(DownloadOutcome::Completed { bytes: total })
}

#[allow(clippy::too_many_arguments)]
async fn download_segment(
    client: &Client,
    url: &str,
    path: &Path,
    start: u64,
    end: u64,
    total: u64,
    existing: u64,
    validator: Option<String>,
    cancel: Arc<AtomicBool>,
    progress: tokio::sync::mpsc::Sender<(u64, usize)>,
    trackers: Arc<tokio::sync::RwLock<Vec<u64>>>,
    index: usize,
) -> Result<(), DownloadError> {
    let expected = end - start + 1;
    let mut last_error = None;

    for attempt in 1..=MAX_ATTEMPTS {
        if cancel.load(Ordering::Relaxed) {
            return Err(DownloadError::Paused);
        }
        let current_existing = {
            let read = trackers.read().await;
            read[index].max(existing).min(expected)
        };
        if current_existing == expected {
            return Ok(());
        }
        let requested_start = start + current_existing;
        let mut request = client
            .get(url)
            .header(RANGE, format!("bytes={requested_start}-{end}"));
        if let Some(validator) = &validator {
            request = request.header(IF_RANGE, validator);
        }
        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                last_error = Some(DownloadError::Network(error));
                retry_delay(attempt).await;
                continue;
            }
        };
        if response.status() != StatusCode::PARTIAL_CONTENT {
            let error = DownloadError::InvalidRange(format!(
                "worker expected HTTP 206, received {}",
                response.status()
            ));
            if response.status() == StatusCode::OK {
                return Err(error);
            }
            last_error = Some(error);
            retry_delay(attempt).await;
            continue;
        }
        let content_range = response
            .headers()
            .get(CONTENT_RANGE)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| DownloadError::InvalidRange("Content-Range missing".into()))?;
        let (received_start, received_total) = parse_content_range(content_range)?;
        if received_start != requested_start || received_total != Some(total) {
            return Err(DownloadError::InvalidRange(format!(
                "worker requested {requested_start}-{end}/{total}, received {content_range}"
            )));
        }

        let mut raw_file = tokio::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .await?;
        raw_file.seek(SeekFrom::Start(requested_start)).await?;

        let mut file = tokio::io::BufWriter::with_capacity(256 * 1024, raw_file);
        let mut response = response;
        let mut written = current_existing;
        let mut unbatched_bytes = 0u64;
        let mut last_progress_send = Instant::now();

        loop {
            if cancel.load(Ordering::Relaxed) {
                if unbatched_bytes > 0 {
                    let _ = progress.send((unbatched_bytes, attempt)).await;
                }
                file.flush().await?;
                let mut lock = trackers.write().await;
                lock[index] = written;
                return Err(DownloadError::Paused);
            }
            match tokio::time::timeout(Duration::from_secs(30), response.chunk()).await {
                Ok(Ok(Some(chunk))) => {
                    let remaining = expected - written;
                    let bytes = chunk.len().min(remaining as usize);
                    file.write_all(&chunk[..bytes]).await?;
                    written += bytes as u64;
                    unbatched_bytes += bytes as u64;
                    if unbatched_bytes >= 256 * 1024
                        || last_progress_send.elapsed() >= Duration::from_millis(100)
                    {
                        let _ = progress.send((unbatched_bytes, attempt)).await;
                        unbatched_bytes = 0;
                        last_progress_send = Instant::now();
                        let mut lock = trackers.write().await;
                        lock[index] = written;
                    }
                    if written == expected {
                        if unbatched_bytes > 0 {
                            let _ = progress.send((unbatched_bytes, attempt)).await;
                        }
                        file.flush().await?;
                        file.get_mut().sync_data().await?;
                        let mut lock = trackers.write().await;
                        lock[index] = written;
                        return Ok(());
                    }
                }
                Ok(Ok(None)) => {
                    if unbatched_bytes > 0 {
                        let _ = progress.send((unbatched_bytes, attempt)).await;
                    }
                    file.flush().await?;
                    let mut lock = trackers.write().await;
                    lock[index] = written;
                    last_error = Some(DownloadError::Incomplete {
                        downloaded: written,
                        expected,
                    });
                    break;
                }
                Ok(Err(error)) => {
                    if unbatched_bytes > 0 {
                        let _ = progress.send((unbatched_bytes, attempt)).await;
                    }
                    file.flush().await?;
                    let mut lock = trackers.write().await;
                    lock[index] = written;
                    last_error = Some(DownloadError::Network(error));
                    break;
                }
                Err(_) => {
                    if unbatched_bytes > 0 {
                        let _ = progress.send((unbatched_bytes, attempt)).await;
                    }
                    file.flush().await?;
                    let mut lock = trackers.write().await;
                    lock[index] = written;
                    last_error = Some(DownloadError::InvalidRange("read timeout".into()));
                    break;
                }
            }
        }
        retry_delay(attempt).await;
    }

    let downloaded = {
        let read = trackers.read().await;
        read[index]
    };
    Err(last_error.unwrap_or(DownloadError::Incomplete {
        downloaded,
        expected,
    }))
}

fn segment_count(total: u64) -> usize {
    if total < 256 * 1024 * 1024 {
        2
    } else if total < 2 * 1024 * 1024 * 1024 {
        4
    } else {
        MAX_SEGMENTS
    }
}

fn segment_ranges(total: u64, segments: usize) -> Vec<(u64, u64)> {
    let size = total / segments as u64;
    (0..segments)
        .map(|index| {
            let start = index as u64 * size;
            let end = if index + 1 == segments {
                total - 1
            } else {
                start + size - 1
            };
            (start, end)
        })
        .collect()
}

fn segment_path(destination: &Path, index: usize) -> PathBuf {
    sidecar_path(destination, &format!("part.{index}"))
}

fn metadata_matches(previous: &ResumeMetadata, current: &ResumeMetadata) -> bool {
    if previous.total != current.total || previous.segments != current.segments {
        return false;
    }

    if let (Some(a), Some(b)) = (&previous.etag, &current.etag) {
        return a == b;
    }

    match (&previous.last_modified, &current.last_modified) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

async fn remove_segment_files(destination: &Path) {
    for index in 0..MAX_SEGMENTS {
        let _ = tokio::fs::remove_file(segment_path(destination, index)).await;
    }
    let _ = tokio::fs::remove_file(sidecar_path(destination, "assembling")).await;
}

fn parse_content_range(value: &str) -> Result<(u64, Option<u64>), DownloadError> {
    let value = value
        .strip_prefix("bytes ")
        .ok_or_else(|| DownloadError::InvalidRange(value.into()))?;
    let (range, total) = value
        .split_once('/')
        .ok_or_else(|| DownloadError::InvalidRange(value.into()))?;
    let (start, _) = range
        .split_once('-')
        .ok_or_else(|| DownloadError::InvalidRange(value.into()))?;
    let start = start
        .parse()
        .map_err(|_| DownloadError::InvalidRange(value.into()))?;
    let total = if total == "*" {
        None
    } else {
        Some(
            total
                .parse()
                .map_err(|_| DownloadError::InvalidRange(value.into()))?,
        )
    };
    Ok((start, total))
}

fn sidecar_path(destination: &Path, suffix: &str) -> PathBuf {
    let mut name = destination.as_os_str().to_os_string();
    name.push(".");
    name.push(suffix);
    PathBuf::from(name)
}

async fn file_len(path: &Path) -> u64 {
    tokio::fs::metadata(path)
        .await
        .map(|metadata| metadata.len())
        .unwrap_or_default()
}

async fn truncate(path: &Path) -> Result<(), std::io::Error> {
    tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .await
        .map(|_| ())
}

async fn finalize(
    partial: &Path,
    metadata: &Path,
    destination: &Path,
) -> Result<(), std::io::Error> {
    if tokio::fs::rename(partial, destination).await.is_err() {
        let _ = tokio::fs::remove_file(destination).await;
        tokio::fs::rename(partial, destination).await?;
    }
    let _ = tokio::fs::remove_file(metadata).await;
    Ok(())
}

async fn read_metadata(path: &Path) -> ResumeMetadata {
    let Ok(bytes) = tokio::fs::read(path).await else {
        return ResumeMetadata::default();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

async fn write_metadata(path: &Path, metadata: &ResumeMetadata) -> Result<(), std::io::Error> {
    let bytes = serde_json::to_vec(metadata).map_err(std::io::Error::other)?;
    tokio::fs::write(path, bytes).await
}

fn header_string(
    response: &reqwest::Response,
    name: reqwest::header::HeaderName,
) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

async fn retry_delay(attempt: usize) {
    if attempt < MAX_ATTEMPTS {
        tokio::time::sleep(Duration::from_secs(1 << (attempt - 1))).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_content_range_valid() {
        let (start, total) = parse_content_range("bytes 0-1023/2048").unwrap();
        assert_eq!(start, 0);
        assert_eq!(total, Some(2048));

        let (start, total) = parse_content_range("bytes 500-999/*").unwrap();
        assert_eq!(start, 500);
        assert_eq!(total, None);
    }

    #[test]
    fn test_parse_content_range_invalid() {
        assert!(parse_content_range("invalid range").is_err());
        assert!(parse_content_range("bytes invalid").is_err());
    }

    #[test]
    fn test_segment_ranges_partitioning() {
        let total = 1000;
        let ranges = segment_ranges(total, 4);
        assert_eq!(ranges.len(), 4);
        assert_eq!(ranges[0], (0, 249));
        assert_eq!(ranges[1], (250, 499));
        assert_eq!(ranges[2], (500, 749));
        assert_eq!(ranges[3], (750, 999));
    }

    #[test]
    fn test_safe_file_stem_empty_fallback() {
        assert_eq!(safe_file_stem(""), DEFAULT_STREAM_NAME);
        assert_eq!(safe_file_stem("   "), DEFAULT_STREAM_NAME);
        assert_eq!(safe_file_stem("..."), DEFAULT_STREAM_NAME);
        assert_eq!(safe_file_stem("___"), DEFAULT_STREAM_NAME);
    }

    #[test]
    fn test_safe_file_stem_sanitization_and_reserved() {
        assert_eq!(safe_file_stem("../../../etc/passwd"), "etc_passwd");
        assert_eq!(
            safe_file_stem("C:\\Windows\\System32\\calc.exe"),
            "C__Windows_System32_calc.exe"
        );
        assert_eq!(safe_file_stem("CON"), "CON_");
        assert_eq!(safe_file_stem("con"), "con_");
        assert_eq!(safe_file_stem("PRN"), "PRN_");
        assert_eq!(safe_file_stem("AUX"), "AUX_");
        assert_eq!(safe_file_stem("NUL"), "NUL_");
        assert_eq!(safe_file_stem("COM1"), "COM1_");
        assert_eq!(safe_file_stem("LPT9"), "LPT9_");
        assert_eq!(
            safe_file_stem("Normal Title: Special"),
            "Normal Title_ Special"
        );
        assert_eq!(
            safe_file_stem("Movie: The <Ultimate> Edition *?|"),
            "Movie_ The _Ultimate_ Edition"
        );
    }

    #[tokio::test]
    async fn test_finalize_replaces_existing_destination_cleanly() {
        let dir = std::env::temp_dir().join(format!(
            "mbx_test_finalize_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let partial = dir.join("video.mp4.part");
        let metadata = dir.join("video.mp4.metadata");
        let destination = dir.join("video.mp4");

        tokio::fs::write(&destination, b"old version")
            .await
            .unwrap();
        tokio::fs::write(&partial, b"new version").await.unwrap();
        tokio::fs::write(&metadata, b"{}").await.unwrap();

        let res = finalize(&partial, &metadata, &destination).await;
        assert!(res.is_ok());
        assert!(!partial.exists());
        assert!(!metadata.exists());
        assert!(destination.exists());
        assert_eq!(
            tokio::fs::read_to_string(&destination).await.unwrap(),
            "new version"
        );
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
    #[test]
    fn test_download_error_user_message() {
        let err_403 = DownloadError::Http(StatusCode::FORBIDDEN);
        assert_eq!(
            err_403.user_message(),
            "Server refused download (HTTP 403)."
        );

        let err_404 = DownloadError::Http(StatusCode::NOT_FOUND);
        assert_eq!(
            err_404.user_message(),
            "File is no longer available on server."
        );

        let err_429 = DownloadError::Http(StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            err_429.user_message(),
            "Server rate limit exceeded. Try again later."
        );

        let err_503 = DownloadError::Http(StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            err_503.user_message(),
            "Server error (HTTP 503). Try again later."
        );

        let err_incomplete = DownloadError::Incomplete {
            downloaded: 10 * 1024 * 1024,
            expected: 20 * 1024 * 1024,
        };
        assert_eq!(
            err_incomplete.user_message(),
            "Download stopped at 10.0 of 20.0 MB."
        );

        let err_paused = DownloadError::Paused;
        assert_eq!(err_paused.user_message(), "Download paused.");
    }

    #[tokio::test]
    async fn test_download_forwards_custom_headers_and_content_to_local_server() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{port}/video.mp4");

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let n = socket.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            let lower = req.to_lowercase();
            assert!(lower.contains("user-agent: moviebox-custom-ua"));
            assert!(lower.contains("referer: https://custom.referer.test/"));
            assert!(lower.contains("x-auth-token: secret-token-123"));
            let body = b"test video binary chunk stream";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: video/mp4\r\nConnection: close\r\n\r\n",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
            socket.write_all(body).await.unwrap();
        });

        let temp_dir = std::env::temp_dir().join(format!("mbx_dl_test_{}", std::process::id()));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();
        let dest_file = temp_dir.join("test_video.mp4");

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            "MovieBox-Custom-UA".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::REFERER,
            "https://custom.referer.test/".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("x-auth-token"),
            "secret-token-123".parse().unwrap(),
        );

        let client = crate::net::streaming_client_builder()
            .default_headers(headers)
            .build()
            .unwrap();

        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut progress_count = 0usize;

        let res = download(&client, &url, &dest_file, cancel, |_prog| {
            progress_count += 1;
        })
        .await;

        server.await.unwrap();
        assert!(matches!(res, Ok(DownloadOutcome::Completed { .. })));
        assert!(dest_file.exists());
        let saved = tokio::fs::read(&dest_file).await.unwrap();
        assert_eq!(saved, b"test video binary chunk stream");
        assert!(progress_count > 0);

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }
}
