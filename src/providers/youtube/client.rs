use crate::providers::models::{
    CatalogItem, MediaDetails, MediaType, ProviderError, ProviderKind, ProviderMediaId, Release,
    SourceMirror,
};
use crate::providers::youtube::parser::{
    extract_youtube_video_id, format_duration_display, parse_upload_year, select_best_thumbnail,
    YtDlpPlaylistSearchOutput, YtDlpVideoDetails,
};
use std::collections::BTreeMap;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Clone, Default)]
pub struct YouTubeClient;

impl YouTubeClient {
    pub fn new() -> Self {
        Self
    }

    pub fn ytdlp_path() -> Result<String, ProviderError> {
        crate::player::find_in_path("yt-dlp").ok_or_else(|| {
            let guide = if cfg!(target_os = "windows") {
                "yt-dlp is required for YouTube streaming & downloads. Install it via: 'winget install yt-dlp.yt-dlp Gyan.FFmpeg'"
            } else if cfg!(target_os = "macos") {
                "yt-dlp is required for YouTube streaming & downloads. Install it via: 'brew install yt-dlp ffmpeg'"
            } else if crate::updater::artifact::is_termux_environment() {
                "yt-dlp is required for YouTube streaming & downloads. Install it via: 'pkg install yt-dlp ffmpeg'"
            } else {
                "yt-dlp is required for YouTube streaming & downloads. Install it via: 'sudo apt install yt-dlp ffmpeg'"
            };
            ProviderError::Unavailable(guide.to_string())
        })
    }

    pub async fn search(&self, query: &str) -> Result<Vec<CatalogItem>, ProviderError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        // 1. If direct YouTube video ID or URL was provided
        if let Some(video_id) = extract_youtube_video_id(trimmed) {
            let poster = format!("https://i.ytimg.com/vi/{video_id}/hqdefault.jpg");
            return Ok(vec![CatalogItem {
                id: ProviderMediaId {
                    provider: ProviderKind::YouTube,
                    value: video_id.clone(),
                },
                title: format!("YouTube Video ({video_id})"),
                media_type: MediaType::Movie,
                year: None,
                poster_url: Some(poster),
                season_count: None,
            }]);
        }

        let ytdlp_bin = Self::ytdlp_path()?;
        let search_target = format!("ytsearch20:{trimmed}");

        let mut cmd = Command::new(ytdlp_bin);
        cmd.arg("--dump-single-json")
            .arg("--flat-playlist")
            .arg("--no-warnings")
            .arg("--ignore-errors")
            .arg(&search_target);

        #[cfg(target_os = "windows")]
        {
            cmd.creation_flags(crate::player::CREATE_NO_WINDOW);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = tokio::time::timeout(std::time::Duration::from_secs(18), cmd.output())
            .await
            .map_err(|_| ProviderError::Network("YouTube search timed out".to_string()))?
            .map_err(|e| ProviderError::Unavailable(format!("Failed to execute yt-dlp: {e}")))?;

        if !output.status.success() && output.stdout.is_empty() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(ProviderError::Network(format!(
                "yt-dlp search failed: {err_msg}"
            )));
        }

        let search_json = String::from_utf8_lossy(&output.stdout);
        let parsed: YtDlpPlaylistSearchOutput = serde_json::from_str(&search_json)
            .map_err(|e| ProviderError::Parsing(format!("Failed to parse YouTube search: {e}")))?;

        let mut items = Vec::new();
        if let Some(entries) = parsed.entries {
            for entry in entries {
                let Some(id) = entry.id else { continue };
                let title = entry.title.unwrap_or_else(|| format!("Video {id}"));
                let year = parse_upload_year(entry.upload_date.as_deref());
                let poster_url = Some(select_best_thumbnail(entry.thumbnails.as_deref(), &id));

                items.push(CatalogItem {
                    id: ProviderMediaId {
                        provider: ProviderKind::YouTube,
                        value: id,
                    },
                    title,
                    media_type: MediaType::Movie,
                    year,
                    poster_url,
                    season_count: None,
                });
            }
        }

        Ok(items)
    }

    pub async fn details(&self, id: &str) -> Result<MediaDetails, ProviderError> {
        let video_id = extract_youtube_video_id(id).unwrap_or_else(|| id.to_string());
        let ytdlp_bin = Self::ytdlp_path()?;
        let target_url = format!("https://www.youtube.com/watch?v={video_id}");

        let mut cmd = Command::new(&ytdlp_bin);
        cmd.arg("--dump-json")
            .arg("--no-playlist")
            .arg("--no-warnings")
            .arg("--extractor-args")
            .arg("youtube:player_client=ios,android,web")
            .arg(&target_url);

        #[cfg(target_os = "windows")]
        {
            cmd.creation_flags(crate::player::CREATE_NO_WINDOW);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = tokio::time::timeout(std::time::Duration::from_secs(20), cmd.output())
            .await
            .map_err(|_| ProviderError::Network("YouTube details request timed out".to_string()))?
            .map_err(|e| ProviderError::Unavailable(format!("Failed to run yt-dlp: {e}")))?;

        let video_details = if output.status.success() && !output.stdout.is_empty() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            serde_json::from_str::<YtDlpVideoDetails>(&json_str).ok()
        } else {
            // Fallback command without extractor args
            let mut fallback_cmd = Command::new(&ytdlp_bin);
            fallback_cmd
                .arg("--dump-json")
                .arg("--no-playlist")
                .arg("--no-warnings")
                .arg(&target_url);

            #[cfg(target_os = "windows")]
            {
                fallback_cmd.creation_flags(crate::player::CREATE_NO_WINDOW);
            }
            fallback_cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

            if let Ok(Ok(fb_out)) = tokio::time::timeout(
                std::time::Duration::from_secs(15),
                fallback_cmd.output(),
            )
            .await
            {
                let json_str = String::from_utf8_lossy(&fb_out.stdout);
                serde_json::from_str::<YtDlpVideoDetails>(&json_str).ok()
            } else {
                None
            }
        };

        if let Some(details) = video_details {
            let title = details
                .title
                .or(details.fulltitle)
                .unwrap_or_else(|| format!("YouTube Video ({video_id})"));
            let year = parse_upload_year(details.upload_date.as_deref());
            let duration = details.duration_string.or_else(|| {
                details.duration.map(format_duration_display)
            });
            let poster_url = details.thumbnail.or_else(|| {
                Some(select_best_thumbnail(details.thumbnails.as_deref(), &video_id))
            });
            let director = details.channel.or(details.uploader);
            let description = details.description;
            let genres = details.categories.unwrap_or_default();

            Ok(MediaDetails {
                id: ProviderMediaId {
                    provider: ProviderKind::YouTube,
                    value: video_id,
                },
                title,
                media_type: MediaType::Movie,
                year,
                description,
                tagline: None,
                imdb_rating: details.like_count.map(|c| format!("{c} likes")),
                director,
                stars: None,
                prints: Some("YouTube (yt-dlp Stream)".to_string()),
                audios: Some("Original Audio".to_string()),
                poster_url,
                duration,
                genres,
                seasons: Vec::new(),
                dubs: Vec::new(),
            })
        } else {
            // Fallback basic metadata if yt-dlp dump-json was throttled or unavailable
            Ok(MediaDetails {
                id: ProviderMediaId {
                    provider: ProviderKind::YouTube,
                    value: video_id.clone(),
                },
                title: format!("YouTube Video ({video_id})"),
                media_type: MediaType::Movie,
                year: None,
                description: Some("Watch directly with yt-dlp stream acceleration.".to_string()),
                tagline: None,
                imdb_rating: None,
                director: Some("YouTube".to_string()),
                stars: None,
                prints: Some("YouTube Stream".to_string()),
                audios: Some("Original Audio".to_string()),
                poster_url: Some(format!("https://i.ytimg.com/vi/{video_id}/hqdefault.jpg")),
                duration: None,
                genres: vec!["YouTube".to_string()],
                seasons: Vec::new(),
                dubs: Vec::new(),
            })
        }
    }

    pub async fn releases(&self, id: &str) -> Result<Vec<Release>, ProviderError> {
        let video_id = extract_youtube_video_id(id).unwrap_or_else(|| id.to_string());
        let ytdlp_bin = Self::ytdlp_path()?;
        let canonical_url = format!("https://www.youtube.com/watch?v={video_id}");

        let mut cmd = Command::new(&ytdlp_bin);
        cmd.arg("--dump-json")
            .arg("--no-playlist")
            .arg("--no-warnings")
            .arg("--extractor-args")
            .arg("youtube:player_client=ios,android,web")
            .arg(&canonical_url);

        #[cfg(target_os = "windows")]
        {
            cmd.creation_flags(crate::player::CREATE_NO_WINDOW);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let raw_details: Option<YtDlpVideoDetails> = if let Ok(Ok(output)) =
            tokio::time::timeout(std::time::Duration::from_secs(20), cmd.output()).await
        {
            if output.status.success() && !output.stdout.is_empty() {
                let json_str = String::from_utf8_lossy(&output.stdout);
                serde_json::from_str(&json_str).ok()
            } else {
                None
            }
        } else {
            None
        };

        let mut releases: Vec<Release> = Vec::new();

        // 1. Primary Release: Multi-Resolution Best Stream
        releases.push(Release {
            provider: ProviderKind::YouTube,
            filename: format!("YouTube - {video_id} (Best Multi-Res)"),
            quality: Some("Multi".to_string()),
            codec: Some("Auto/yt-dlp".to_string()),
            language: Some("Original".to_string()),
            size_bytes: None,
            season: None,
            episode: None,
            mirrors: vec![
                SourceMirror {
                    label: "YouTube (yt-dlp Stream)".to_string(),
                    resolver_url: canonical_url.clone(),
                    headers: vec![("User-Agent".to_string(), "Mozilla/5.0".to_string())],
                    direct_file: false,
                },
            ],
            resource_id: Some(video_id.clone()),
        });

        // 2. Parse individual format resolutions if available
        if let Some(details) = raw_details {
            if let Some(formats) = details.formats {
                // Group formats by height (e.g. 2160, 1440, 1080, 720, 480, 360)
                let mut progressive_by_height: BTreeMap<u64, &crate::providers::youtube::parser::YtDlpFormat> = BTreeMap::new();
                let mut best_audio: Option<&crate::providers::youtube::parser::YtDlpFormat> = None;

                for fmt in &formats {
                    let has_vcodec = fmt.vcodec.as_deref().is_some_and(|c| c != "none" && !c.is_empty());
                    let has_acodec = fmt.acodec.as_deref().is_some_and(|c| c != "none" && !c.is_empty());

                    if let Some(h) = fmt.height {
                        if h > 0 && (has_vcodec || fmt.url.is_some()) {
                            progressive_by_height.insert(h, fmt);
                        }
                    } else if !has_vcodec && has_acodec {
                        best_audio = Some(fmt);
                    }
                }

                // Add discrete resolution options in descending order
                for (height, fmt) in progressive_by_height.iter().rev() {
                    let quality_label = format!("{height}p");
                    let stream_url = fmt.url.clone().or_else(|| fmt.manifest_url.clone()).unwrap_or_else(|| canonical_url.clone());
                    let mut headers_vec = Vec::new();
                    if let Some(ref hdrs) = fmt.http_headers {
                        for (k, v) in hdrs {
                            headers_vec.push((k.clone(), v.clone()));
                        }
                    }
                    if headers_vec.is_empty() {
                        headers_vec.push(("User-Agent".to_string(), "Mozilla/5.0".to_string()));
                    }

                    releases.push(Release {
                        provider: ProviderKind::YouTube,
                        filename: format!("YouTube - {video_id} ({quality_label})"),
                        quality: Some(quality_label),
                        codec: fmt.vcodec.clone().or_else(|| Some("H.264/VP9".to_string())),
                        language: Some("Original".to_string()),
                        size_bytes: fmt.filesize.or(fmt.filesize_approx),
                        season: None,
                        episode: None,
                        mirrors: vec![SourceMirror {
                            label: format!("YouTube {height}p Direct"),
                            resolver_url: stream_url,
                            headers: headers_vec,
                            direct_file: true,
                        }],
                        resource_id: Some(video_id.clone()),
                    });
                }

                // Add Audio Only option if audio stream exists
                if let Some(audio_fmt) = best_audio {
                    let stream_url = audio_fmt.url.clone().unwrap_or_else(|| canonical_url.clone());
                    releases.push(Release {
                        provider: ProviderKind::YouTube,
                        filename: format!("YouTube - {video_id} (Audio Only)"),
                        quality: Some("Audio".to_string()),
                        codec: audio_fmt.acodec.clone().or_else(|| Some("M4A/Opus".to_string())),
                        language: Some("Original".to_string()),
                        size_bytes: audio_fmt.filesize.or(audio_fmt.filesize_approx),
                        season: None,
                        episode: None,
                        mirrors: vec![SourceMirror {
                            label: "YouTube Audio Stream".to_string(),
                            resolver_url: stream_url,
                            headers: vec![("User-Agent".to_string(), "Mozilla/5.0".to_string())],
                            direct_file: true,
                        }],
                        resource_id: Some(video_id.clone()),
                    });
                }
            }
        }

        Ok(releases)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::Provider;

    #[test]
    fn test_youtube_client_provider_id_and_caps() {
        let client = YouTubeClient::new();
        assert_eq!(client.id(), ProviderKind::YouTube);
        let caps = client.capabilities();
        assert!(caps.supports_search);
        assert!(!caps.supports_series);
        assert!(!caps.supports_homepage);
    }

    #[tokio::test]
    async fn test_youtube_direct_id_search() {
        let client = YouTubeClient::new();
        let items = client.search("dQw4w9WgXcQ").await.expect("search direct id");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id.value, "dQw4w9WgXcQ");
        assert_eq!(items[0].id.provider, ProviderKind::YouTube);
    }
}
