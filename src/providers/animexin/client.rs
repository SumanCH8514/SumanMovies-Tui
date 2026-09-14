use super::parser;
use crate::providers::models::{CatalogItem, MediaDetails, PlaybackSource, ProviderKind, Release};
use reqwest::Url;
use scraper::{Html, Selector};

const DEFAULT_BASE_URL: &str = "https://animexin.dev/";
const BROWSER_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

#[derive(thiserror::Error, Debug)]
pub enum AnimeXinError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("invalid provider URL: {0}")]
    InvalidUrl(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("no playable mirrors: {0}")]
    NoPlayableMirror(String),
}

#[derive(Clone)]
pub struct AnimeXinClient {
    client: reqwest::Client,
    base_url: Url,
}

impl AnimeXinClient {
    pub fn new() -> Result<Self, AnimeXinError> {
        let base = std::env::var("SUMANMOVIES_ANIMEXIN_URL")
            .or_else(|_| std::env::var("MOVIEBOX_ANIMEXIN_URL"))
            .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        Self::with_base_url(&base)
    }

    pub fn with_base_url(base: &str) -> Result<Self, AnimeXinError> {
        let base_url = Url::parse(base).map_err(|_| AnimeXinError::InvalidUrl(base.to_string()))?;
        if base_url.scheme() != "https" && base_url.scheme() != "http" {
            return Err(AnimeXinError::InvalidUrl(base.to_string()));
        }
        Ok(Self {
            client: build_client(),
            base_url,
        })
    }

    pub async fn health_check(&self) -> Result<(), AnimeXinError> {
        let response = self.client.get(self.base_url.clone()).send().await?;
        if !response.status().is_success() {
            return Err(AnimeXinError::Parse(format!(
                "health check returned {}",
                response.status()
            )));
        }
        Ok(())
    }

    pub async fn search(&self, query: &str, page: usize) -> Result<Vec<CatalogItem>, AnimeXinError> {
        let mut url = self.base_url.clone();
        if page > 1 {
            if let Ok(paged_url) = self.base_url.join(&format!("page/{page}/")) {
                url = paged_url;
            }
        }
        url.query_pairs_mut().append_pair("s", query);
        let html = self.fetch_text(url).await?;
        parser::parse_search(&self.base_url, &html)
    }

    pub async fn details(&self, id: &str) -> Result<MediaDetails, AnimeXinError> {
        let url = self.provider_url(id)?;
        let html = self.fetch_text(url).await?;
        parser::parse_details(id, &html)
    }

    pub async fn releases(
        &self,
        id: &str,
        season: usize,
        episode: usize,
    ) -> Result<Vec<Release>, AnimeXinError> {
        let ep_html = if id.contains("-episode-") {
            let url = self.provider_url(id)?;
            self.fetch_text(url).await?
        } else {
            let series_url = self.provider_url(id)?;
            let series_html = self.fetch_text(series_url).await?;
            let ep_path = find_episode_link(&series_html, episode).unwrap_or_else(|| {
                let clean_id = id.trim_matches('/');
                format!("{clean_id}-episode-{episode}-indonesia-english-sub")
            });
            let url = self.provider_url(&ep_path)?;
            self.fetch_text(url).await?
        };

        parser::parse_releases(&ep_html, season, episode)
    }

    pub async fn resolve_release(
        &self,
        release: &Release,
    ) -> Result<PlaybackSource, AnimeXinError> {
        if release.provider != ProviderKind::AnimeXin {
            return Err(AnimeXinError::Parse(
                "release belongs to another provider".into(),
            ));
        }

        let Some(first_mirror) = release.mirrors.first() else {
            return Err(AnimeXinError::NoPlayableMirror(
                "no mirrors available for this release".into(),
            ));
        };

        let raw_url = &first_mirror.resolver_url;

        // 1. Mediafire resolver: directly extract the fast streamable .mp4 link
        if raw_url.contains("mediafire.com") {
            match self.resolve_mediafire(raw_url).await {
                Ok(stream_url) => {
                    log::info!("Successfully resolved Mediafire stream URL: {stream_url}");
                    return Ok(PlaybackSource {
                        provider: ProviderKind::AnimeXin,
                        url: stream_url,
                        headers: vec![("User-Agent".to_string(), BROWSER_UA.to_string())],
                        subtitle: None,
                        source_label: first_mirror.label.clone(),
                    });
                }
                Err(e) => {
                    log::warn!("Mediafire direct extraction failed: {e:?}, using raw url");
                }
            }
        }

        // 2. Mirrored.to resolver: follow user workflow through mirrored.to -> GoFile / VikingFile
        if raw_url.contains("mirrored.to") {
            match self.resolve_mirrored(raw_url).await {
                Ok(stream_url) => {
                    log::info!("Successfully resolved Mirrored.to URL: {stream_url}");
                    // If mirrored.to pointed to a Mediafire link, resolve that directly
                    if stream_url.contains("mediafire.com") {
                        if let Ok(mf_url) = self.resolve_mediafire(&stream_url).await {
                            return Ok(PlaybackSource {
                                provider: ProviderKind::AnimeXin,
                                url: mf_url,
                                headers: vec![("User-Agent".to_string(), BROWSER_UA.to_string())],
                                subtitle: None,
                                source_label: first_mirror.label.clone(),
                            });
                        }
                    }

                    return Ok(PlaybackSource {
                        provider: ProviderKind::AnimeXin,
                        url: stream_url,
                        headers: vec![("User-Agent".to_string(), BROWSER_UA.to_string())],
                        subtitle: None,
                        source_label: first_mirror.label.clone(),
                    });
                }
                Err(e) => {
                    log::warn!("Mirrored.to workflow failed: {e:?}, using raw url");
                }
            }
        }

        // 3. Attempt stream extraction via yt-dlp for embedded hosts (Rumble, Odysee, Ok.ru, Dailymotion, etc.)
        if let Some(ytdlp) = crate::player::find_in_path("yt-dlp") {
            if raw_url.contains("rumble.com")
                || raw_url.contains("odysee.com")
                || raw_url.contains("dailymotion.com")
                || raw_url.contains("ok.ru")
            {
                let mut cmd = tokio::process::Command::new(ytdlp);
                cmd.args(["-g", "--no-warnings", raw_url]);
                #[cfg(windows)]
                {
                    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
                }

                if let Ok(Ok(out)) = tokio::time::timeout(
                    std::time::Duration::from_secs(8),
                    cmd.output(),
                )
                .await
                {
                    if out.status.success() {
                        let stream_url = String::from_utf8_lossy(&out.stdout)
                            .lines()
                            .next()
                            .unwrap_or_default()
                            .trim()
                            .to_string();
                        if !stream_url.is_empty() && stream_url.starts_with("http") {
                            return Ok(PlaybackSource {
                                provider: ProviderKind::AnimeXin,
                                url: stream_url,
                                headers: Vec::new(),
                                subtitle: None,
                                source_label: first_mirror.label.clone(),
                            });
                        }
                    }
                }
            }
        }

        // Return direct resolver URL as fallback
        Ok(PlaybackSource {
            provider: ProviderKind::AnimeXin,
            url: raw_url.clone(),
            headers: vec![("User-Agent".to_string(), BROWSER_UA.to_string())],
            subtitle: None,
            source_label: first_mirror.label.clone(),
        })
    }

    pub async fn resolve_mediafire(&self, url: &str) -> Result<String, AnimeXinError> {
        let resp = self
            .client
            .get(url)
            .header(reqwest::header::USER_AGENT, BROWSER_UA)
            .header(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
            .send()
            .await?;
        let html = resp.text().await?;

        // 1. Scraper selector search
        let document = Html::parse_document(&html);
        if let Ok(btn_sel) = Selector::parse("#downloadButton, a[aria-label='Download file']") {
            for el in document.select(&btn_sel) {
                if let Some(href) = el.value().attr("href") {
                    let href_clean = href.trim();
                    if href_clean.starts_with("http") {
                        return Ok(href_clean.to_string());
                    }
                }
            }
        }

        // 2. String fallback for direct download link (e.g. https://download2443.mediafire.com/...)
        if let Some(idx) = html.find("https://download") {
            let rest = &html[idx..];
            let end = rest
                .find(|c: char| c == '"' || c == '\'' || c.is_whitespace() || c == '<' || c == '>')
                .unwrap_or(rest.len());
            let direct = rest[..end].replace("&amp;", "&");
            return Ok(direct);
        }

        Err(AnimeXinError::NoPlayableMirror(
            "Mediafire direct download button not found".into(),
        ))
    }

    pub async fn resolve_mirrored(&self, url: &str) -> Result<String, AnimeXinError> {
        // Step 1: Fetch initial page (e.g. https://www.mirrored.to/files/<uid>/<fn>_links)
        let resp1 = self
            .client
            .get(url)
            .header(reqwest::header::USER_AGENT, BROWSER_UA)
            .header(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
            .send()
            .await?;
        let html1 = resp1.text().await?;

        // Extract the hash URL ("Click to view the download links")
        let step2_url = {
            let doc1 = Html::parse_document(&html1);
            let mut found = None;
            if let Ok(a_sel) = Selector::parse("a[href*='/files/'][href*='hash=']") {
                for a in doc1.select(&a_sel) {
                    if let Some(href) = a.value().attr("href") {
                        let full = if href.starts_with("http") {
                            href.to_string()
                        } else {
                            format!(
                                "https://www.mirrored.to{}",
                                if href.starts_with('/') { "" } else { "/" }
                            ) + href
                        };
                        found = Some(full);
                        break;
                    }
                }
            }
            if let Some(u) = found {
                u
            } else if url.contains("hash=") {
                url.to_string()
            } else {
                return Err(AnimeXinError::NoPlayableMirror(
                    "Could not find Mirrored.to hash button".into(),
                ));
            }
        };

        // Step 2: Fetch hash page to extract /mirstats.php endpoint
        let resp2 = self
            .client
            .get(&step2_url)
            .header(reqwest::header::USER_AGENT, BROWSER_UA)
            .header(reqwest::header::REFERER, url)
            .send()
            .await?;
        let html2 = resp2.text().await?;

        let mirstats_path = if let Some(idx) = html2.find("/mirstats.php?") {
            let rest = &html2[idx..];
            let end = rest
                .find(|c: char| c == '"' || c == '\'' || c.is_whitespace() || c == '<' || c == '>')
                .unwrap_or(rest.len());
            &rest[..end]
        } else {
            return Err(AnimeXinError::NoPlayableMirror(
                "Could not find Mirrored.to mirstats endpoint".into(),
            ));
        };

        let mirstats_url = format!("https://www.mirrored.to{mirstats_path}");

        // Step 3: Fetch /mirstats.php to get table of hosts (GoFile, VikingFile, etc.)
        let resp3 = self
            .client
            .get(&mirstats_url)
            .header(reqwest::header::USER_AGENT, BROWSER_UA)
            .header(reqwest::header::REFERER, &step2_url)
            .send()
            .await?;
        let html3 = resp3.text().await?;

        // Parse table rows to find GoFile (host 137) or VikingFile (host 191)
        let target_getlink = {
            let doc3 = Html::parse_document(&html3);
            let tr_sel = Selector::parse("tr").map_err(|e| AnimeXinError::Parse(format!("{e:?}")))?;
            let a_sel = Selector::parse("a[href*='/getlink/']")
                .map_err(|e| AnimeXinError::Parse(format!("{e:?}")))?;

            let mut gofile_link = None;
            let mut viking_link = None;
            let mut any_link = None;

            for tr in doc3.select(&tr_sel) {
                let row_html = tr.html().to_ascii_lowercase();
                if let Some(a) = tr.select(&a_sel).next() {
                    if let Some(href) = a.value().attr("href") {
                        let full_link = if href.starts_with("http") {
                            href.to_string()
                        } else {
                            format!("https://www.mirrored.to{href}")
                        };

                        if row_html.contains("gofile") {
                            gofile_link = Some(full_link.clone());
                        } else if row_html.contains("viking") {
                            viking_link = Some(full_link.clone());
                        }
                        if any_link.is_none() {
                            any_link = Some(full_link);
                        }
                    }
                }
            }

            gofile_link
                .or(viking_link)
                .or(any_link)
                .ok_or_else(|| {
                    AnimeXinError::NoPlayableMirror(
                        "No valid host links found in Mirrored.to table".into(),
                    )
                })?
        };

        // Step 4: Fetch /getlink/... page to extract final host URL (GoFile / VikingFile)
        let resp4 = self
            .client
            .get(&target_getlink)
            .header(reqwest::header::USER_AGENT, BROWSER_UA)
            .header(reqwest::header::REFERER, &mirstats_url)
            .send()
            .await?;
        let html4 = resp4.text().await?;

        let doc4 = Html::parse_document(&html4);
        if let Ok(a_sel) = Selector::parse("a[href]") {
            for a in doc4.select(&a_sel) {
                let a_html = a.html();
                if a_html.contains("get_btn") || a_html.contains("Download from") {
                    if let Some(href) = a.value().attr("href") {
                        if href.starts_with("http") {
                            return Ok(href.to_string());
                        }
                    }
                }
            }
        }

        // Fallback: search for any gofile.io or vikingfile link in page
        for prefix in &[
            "https://gofile.io/d/",
            "http://gofile.io/d/",
            "https://vikingfile.com/f/",
            "https://vik1ngfile.site/f/",
        ] {
            if let Some(idx) = html4.find(prefix) {
                let rest = &html4[idx..];
                let end = rest
                    .find(|c: char| c == '"' || c == '\'' || c.is_whitespace() || c == '<' || c == '>')
                    .unwrap_or(rest.len());
                return Ok(rest[..end].to_string());
            }
        }

        Err(AnimeXinError::NoPlayableMirror(
            "Could not extract final download URL from Mirrored.to getlink page".into(),
        ))
    }

    pub fn provider_url(&self, relative_or_absolute: &str) -> Result<Url, AnimeXinError> {
        let trimmed = relative_or_absolute.trim();
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            let parsed =
                Url::parse(trimmed).map_err(|_| AnimeXinError::InvalidUrl(trimmed.to_string()))?;
            let host_match = parsed.host_str() == self.base_url.host_str();
            if !host_match {
                return Err(AnimeXinError::InvalidUrl(format!(
                    "host mismatch: expected {:?}, got {:?}",
                    self.base_url.host_str(),
                    parsed.host_str()
                )));
            }
            return Ok(parsed);
        }

        let relative = trimmed.trim_start_matches('/');
        let url = self
            .base_url
            .join(relative)
            .map_err(|_| AnimeXinError::InvalidUrl(relative.to_string()))?;
        Ok(url)
    }

    async fn fetch_text(&self, url: Url) -> Result<String, AnimeXinError> {
        let response = self
            .client
            .get(url)
            .header(reqwest::header::USER_AGENT, BROWSER_UA)
            .header(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
            .send()
            .await?
            .error_for_status()?;
        let text = response.text().await?;
        Ok(text)
    }
}

fn find_episode_link(html: &str, target_ep: usize) -> Option<String> {
    let document = Html::parse_document(html);
    let eplister_sel = Selector::parse(".eplister ul li, .eplister li").ok()?;
    let num_sel = Selector::parse(".epl-num").ok()?;
    let link_sel = Selector::parse("a[href]").ok()?;

    for li in document.select(&eplister_sel) {
        let ep_num = li
            .select(&num_sel)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .and_then(|t| {
                t.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<usize>()
                    .ok()
            })?;

        if ep_num == target_ep {
            let href = li.select(&link_sel).next()?.value().attr("href")?;
            let ep_path = href
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .trim_start_matches("animexin.dev/")
                .trim_matches('/')
                .to_string();
            return Some(ep_path);
        }
    }
    None
}

fn build_client() -> reqwest::Client {
    crate::net::http_client_builder()
        .timeout(std::time::Duration::from_secs(20))
        .connect_timeout(std::time::Duration::from_secs(5))
        .user_agent(BROWSER_UA)
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_provider_url() {
        let client = AnimeXinClient::new().expect("client creation");
        assert!(client.provider_url("renegade-immortal").is_ok());
        assert!(client.provider_url("https://evil.com/anime").is_err());
    }
}
