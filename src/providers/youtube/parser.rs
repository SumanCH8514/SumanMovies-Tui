use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct YtDlpThumbnail {
    pub url: Option<String>,
    pub width: Option<u64>,
    pub height: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct YtDlpFormat {
    pub format_id: Option<String>,
    pub format_note: Option<String>,
    pub url: Option<String>,
    pub manifest_url: Option<String>,
    pub ext: Option<String>,
    pub resolution: Option<String>,
    pub width: Option<u64>,
    pub height: Option<u64>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub tbr: Option<f64>,
    pub abr: Option<f64>,
    pub vbr: Option<f64>,
    pub filesize: Option<u64>,
    pub filesize_approx: Option<u64>,
    pub protocol: Option<String>,
    pub http_headers: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct YtDlpSubtitleTrack {
    pub ext: Option<String>,
    pub url: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct YtDlpSearchResultEntry {
    pub id: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub duration: Option<f64>,
    pub uploader: Option<String>,
    pub channel: Option<String>,
    pub view_count: Option<u64>,
    pub upload_date: Option<String>,
    pub thumbnails: Option<Vec<YtDlpThumbnail>>,
    pub formats: Option<Vec<YtDlpFormat>>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct YtDlpPlaylistSearchOutput {
    pub entries: Option<Vec<YtDlpSearchResultEntry>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct YtDlpVideoDetails {
    pub id: Option<String>,
    pub title: Option<String>,
    pub fulltitle: Option<String>,
    pub description: Option<String>,
    pub duration: Option<f64>,
    pub duration_string: Option<String>,
    pub uploader: Option<String>,
    pub channel: Option<String>,
    pub upload_date: Option<String>,
    pub view_count: Option<u64>,
    pub like_count: Option<u64>,
    pub thumbnail: Option<String>,
    pub thumbnails: Option<Vec<YtDlpThumbnail>>,
    pub categories: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub formats: Option<Vec<YtDlpFormat>>,
    pub subtitles: Option<std::collections::HashMap<String, Vec<YtDlpSubtitleTrack>>>,
    pub automatic_captions: Option<std::collections::HashMap<String, Vec<YtDlpSubtitleTrack>>>,
}

pub fn extract_youtube_video_id(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.len() == 11
        && !trimmed.contains('/')
        && !trimmed.contains('?')
        && !trimmed.contains('&')
        && !trimmed.contains(' ')
        && !trimmed.contains(':')
    {
        return Some(trimmed.to_string());
    }

    let url_str = if !trimmed.contains("://") {
        format!("https://{trimmed}")
    } else {
        trimmed.to_string()
    };

    if let Ok(parsed) = url::Url::parse(&url_str) {
        if let Some(host) = parsed.host_str() {
            if host.contains("youtube.com") {
                if parsed.path().starts_with("/watch") {
                    for (k, v) in parsed.query_pairs() {
                        if k == "v" && v.len() == 11 {
                            return Some(v.into_owned());
                        }
                    }
                } else if parsed.path().starts_with("/shorts/") {
                    let id = parsed.path().trim_start_matches("/shorts/").split('/').next()?;
                    let clean_id = id.split('?').next().unwrap_or(id).split('&').next().unwrap_or(id);
                    if clean_id.len() == 11 {
                        return Some(clean_id.to_string());
                    }
                } else if parsed.path().starts_with("/embed/") {
                    let id = parsed.path().trim_start_matches("/embed/").split('/').next()?;
                    let clean_id = id.split('?').next().unwrap_or(id).split('&').next().unwrap_or(id);
                    if clean_id.len() == 11 {
                        return Some(clean_id.to_string());
                    }
                }
            } else if host.contains("youtu.be") {
                let id = parsed.path().trim_start_matches('/').split('/').next()?;
                let clean_id = id.split('?').next().unwrap_or(id).split('&').next().unwrap_or(id);
                if clean_id.len() == 11 {
                    return Some(clean_id.to_string());
                }
            }
        }
    }
    None
}

pub fn format_duration_display(seconds: f64) -> String {
    let total_secs = seconds.round() as u64;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;

    if hours > 0 {
        format!("{hours}:{mins:02}:{secs:02}")
    } else {
        format!("{mins}:{secs:02}")
    }
}

pub fn parse_upload_year(upload_date: Option<&str>) -> Option<String> {
    let date_str = upload_date?.trim();
    if date_str.len() >= 4 {
        let year_part = &date_str[0..4];
        if year_part.chars().all(|c| c.is_ascii_digit()) {
            return Some(year_part.to_string());
        }
    }
    None
}

pub fn detect_youtube_media_type(
    title: &str,
    categories: Option<&[String]>,
    channel: Option<&str>,
) -> crate::providers::models::MediaType {
    if let Some(cats) = categories {
        if cats.iter().any(|c| c.eq_ignore_ascii_case("Music")) {
            return crate::providers::models::MediaType::Music;
        }
    }
    let lower_title = title.to_ascii_lowercase();
    if lower_title.contains("official audio")
        || lower_title.contains("official music")
        || lower_title.contains("lyric video")
        || lower_title.contains("lyrics video")
        || lower_title.contains("(audio)")
        || lower_title.contains("[audio]")
        || lower_title.contains("full album")
        || lower_title.contains("original soundtrack")
        || lower_title.contains("ost")
    {
        return crate::providers::models::MediaType::Music;
    }
    if let Some(ch) = channel {
        let lower_ch = ch.to_ascii_lowercase();
        if lower_ch.ends_with(" - topic")
            || lower_ch.contains("vevo")
            || lower_ch.contains("records")
            || lower_ch.contains("music")
        {
            return crate::providers::models::MediaType::Music;
        }
    }
    crate::providers::models::MediaType::Video
}

pub fn select_best_thumbnail(thumbnails: Option<&[YtDlpThumbnail]>, fallback_id: &str) -> String {
    if let Some(thumbs) = thumbnails {
        if let Some(best) = thumbs
            .iter()
            .filter(|t| {
                t.url.as_ref().map_or(false, |u| {
                    !u.is_empty() && !u.contains("/sb/") && !u.contains("mhtml")
                })
            })
            .max_by_key(|t| {
                let w = t.width.unwrap_or(0);
                let h = t.height.unwrap_or(0);
                if w > 0 && h > 0 {
                    w * h
                } else {
                    1
                }
            })
            .and_then(|t| t.url.as_deref())
        {
            return best.to_string();
        }
    }
    format!("https://i.ytimg.com/vi/{fallback_id}/hqdefault.jpg")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_youtube_video_id() {
        assert_eq!(
            extract_youtube_video_id("dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            extract_youtube_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            extract_youtube_video_id("https://youtu.be/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            extract_youtube_video_id("https://www.youtube.com/shorts/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            extract_youtube_video_id("https://www.youtube.com/embed/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(extract_youtube_video_id("interstellar trailer"), None);
    }

    #[test]
    fn test_format_duration_display() {
        assert_eq!(format_duration_display(65.0), "1:05");
        assert_eq!(format_duration_display(3665.0), "1:01:05");
        assert_eq!(format_duration_display(50350.0), "13:59:10");
    }

    #[test]
    fn test_parse_upload_year() {
        assert_eq!(parse_upload_year(Some("20240518")), Some("2024".to_string()));
        assert_eq!(parse_upload_year(Some("2009")), Some("2009".to_string()));
        assert_eq!(parse_upload_year(None), None);
    }

    #[test]
    fn test_detect_youtube_media_type() {
        use crate::providers::models::MediaType;

        assert_eq!(
            detect_youtube_media_type("Alan Walker - Faded (Official Music Video)", None, None),
            MediaType::Music
        );
        assert_eq!(
            detect_youtube_media_type("Taylor Swift - Fortnight (feat. Post Malone) (Official Audio)", None, None),
            MediaType::Music
        );
        assert_eq!(
            detect_youtube_media_type("Song Name", Some(&["Music".to_string()]), None),
            MediaType::Music
        );
        assert_eq!(
            detect_youtube_media_type("Track", None, Some("Artist - Topic")),
            MediaType::Music
        );
        assert_eq!(
            detect_youtube_media_type("Rust Programming Tutorial 2026", None, Some("Tech With Tim")),
            MediaType::Video
        );
    }

    #[test]
    fn test_select_best_thumbnail() {
        let thumbs = vec![
            YtDlpThumbnail {
                url: Some("https://i.ytimg.com/vi/test/low.jpg".to_string()),
                width: Some(360),
                height: Some(202),
            },
            YtDlpThumbnail {
                url: Some("https://i.ytimg.com/sb/storyboard.mhtml".to_string()),
                width: Some(1920),
                height: Some(1080),
            },
            YtDlpThumbnail {
                url: Some("https://i.ytimg.com/vi/test/hq720.jpg".to_string()),
                width: Some(720),
                height: Some(404),
            },
        ];
        let best = select_best_thumbnail(Some(&thumbs), "test");
        assert_eq!(best, "https://i.ytimg.com/vi/test/hq720.jpg");

        let fallback = select_best_thumbnail(None, "abc123xyz");
        assert_eq!(fallback, "https://i.ytimg.com/vi/abc123xyz/hqdefault.jpg");
    }
}
