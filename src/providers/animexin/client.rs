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

        // Attempt stream extraction via yt-dlp for embedded hosts (Rumble, Odysee, Ok.ru, Dailymotion, etc.)
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

        // Return direct resolver URL (e.g. embed or mediafire direct link)
        Ok(PlaybackSource {
            provider: ProviderKind::AnimeXin,
            url: raw_url.clone(),
            headers: Vec::new(),
            subtitle: None,
            source_label: first_mirror.label.clone(),
        })
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
