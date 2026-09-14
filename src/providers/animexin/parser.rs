use super::client::AnimeXinError;
use crate::providers::models::{
    CatalogItem, Episode, MediaDetails, MediaType, ProviderKind, ProviderMediaId, Release, Season,
    SourceMirror,
};
use base64::Engine;
use scraper::{Html, Selector};
use url::Url;

pub fn parse_search(base: &Url, html: &str) -> Result<Vec<CatalogItem>, AnimeXinError> {
    let document = Html::parse_document(html);
    let article_sel = Selector::parse(".listupd .bsx, .bsx").map_err(|e| {
        AnimeXinError::Parse(format!("failed to parse article selector: {e:?}"))
    })?;
    let link_sel = Selector::parse("a[href]").map_err(|e| {
        AnimeXinError::Parse(format!("failed to parse link selector: {e:?}"))
    })?;
    let title_sel = Selector::parse(".tt h2, h2[itemprop='headline'], .tt, .title").map_err(|e| {
        AnimeXinError::Parse(format!("failed to parse title selector: {e:?}"))
    })?;
    let img_sel = Selector::parse("img[src], img[data-src]").map_err(|e| {
        AnimeXinError::Parse(format!("failed to parse img selector: {e:?}"))
    })?;
    let type_sel = Selector::parse(".typez").map_err(|e| {
        AnimeXinError::Parse(format!("failed to parse type selector: {e:?}"))
    })?;

    let mut seen_ids = std::collections::HashSet::new();
    let mut items = Vec::new();
    for article in document.select(&article_sel) {
        let Some(link_el) = article.select(&link_sel).next() else {
            continue;
        };
        let Some(raw_href) = link_el.value().attr("href") else {
            continue;
        };

        let item_url = match base.join(raw_href) {
            Ok(u) => u,
            Err(_) => continue,
        };

        let title = link_el
            .value()
            .attr("title")
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .or_else(|| {
                article
                    .select(&title_sel)
                    .next()
                    .map(|el| el.text().collect::<String>().trim().to_string())
                    .filter(|t| !t.is_empty())
            })
            .unwrap_or_else(|| "Unknown Title".to_string());

        let poster = article
            .select(&img_sel)
            .next()
            .and_then(|el| el.value().attr("data-src").or_else(|| el.value().attr("src")))
            .map(|src| {
                if src.starts_with("//") {
                    format!("https:{src}")
                } else {
                    src.to_string()
                }
            });

        let type_text = article
            .select(&type_sel)
            .next()
            .map(|el| el.text().collect::<String>().to_ascii_lowercase())
            .unwrap_or_default();

        let media_type = if type_text.contains("movie") {
            MediaType::Movie
        } else {
            MediaType::Series
        };

        let path = item_url.path().trim_matches('/');
        if path.is_empty() || !seen_ids.insert(path.to_string()) {
            continue;
        }

        items.push(CatalogItem {
            id: ProviderMediaId {
                provider: ProviderKind::AnimeXin,
                value: path.to_string(),
            },
            title,
            media_type,
            year: None,
            poster_url: poster,
            season_count: None,
        });
    }

    Ok(items)
}

pub fn parse_details(id: &str, html: &str) -> Result<MediaDetails, AnimeXinError> {
    let document = Html::parse_document(html);

    let title_sel = Selector::parse(".infox h1.entry-title, h1.entry-title, h1").map_err(|e| {
        AnimeXinError::Parse(format!("selector error: {e:?}"))
    })?;
    let title = document
        .select(&title_sel)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
        .filter(|t| !t.is_empty())
        .ok_or_else(|| AnimeXinError::Parse("title missing from AnimeXin details".into()))?;

    let poster_sel = Selector::parse(".thumbook .thumb img, .bigcover img, .thumb img").map_err(|e| {
        AnimeXinError::Parse(format!("selector error: {e:?}"))
    })?;
    let poster_url = document
        .select(&poster_sel)
        .next()
        .and_then(|el| el.value().attr("data-src").or_else(|| el.value().attr("src")))
        .map(|src| {
            if src.starts_with("//") {
                format!("https:{src}")
            } else {
                src.to_string()
            }
        });

    let rating_sel = Selector::parse(".rating strong, .rating-prc").ok();
    let rating = rating_sel.and_then(|sel| {
        document.select(&sel).next().and_then(|el| {
            let text = el.text().collect::<String>();
            text.split_whitespace()
                .find_map(|word| word.parse::<f64>().ok())
        })
    });

    let genres_sel = Selector::parse(".genxed a").ok();
    let genres = genres_sel
        .map(|sel| {
            document
                .select(&sel)
                .map(|el| el.text().collect::<String>().trim().to_string())
                .filter(|g| !g.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let desc_sel = Selector::parse(".synp .entry-content, .infox .desc, .mindesc").ok();
    let overview = desc_sel.and_then(|sel| {
        document.select(&sel).next().map(|el| {
            el.text().collect::<String>().trim().to_string()
        })
    });

    let eplister_sel = Selector::parse(".eplister ul li, .eplister li").ok();
    let num_sel = Selector::parse(".epl-num").ok();
    let ep_title_sel = Selector::parse(".epl-title").ok();
    let ep_link_sel = Selector::parse("a[href]").ok();

    let mut episodes = Vec::new();
    if let (Some(li_sel), Some(n_sel), Some(l_sel)) = (eplister_sel, num_sel, ep_link_sel) {
        for li in document.select(&li_sel) {
            if li.select(&l_sel).next().is_none() {
                continue;
            }

            let ep_num = li
                .select(&n_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .and_then(|t| {
                    t.chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .parse::<usize>()
                        .ok()
                })
                .unwrap_or_else(|| episodes.len() + 1);

            let ep_title = ep_title_sel
                .as_ref()
                .and_then(|ts| li.select(ts).next())
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_else(|| format!("Episode {ep_num}"));

            episodes.push(Episode {
                season: 1,
                number: ep_num,
                title: Some(ep_title),
            });
        }
    }

    episodes.sort_by_key(|e| e.number);

    let seasons = if episodes.is_empty() {
        Vec::new()
    } else {
        vec![Season {
            number: 1,
            episodes,
        }]
    };

    let media_type = if seasons.is_empty() {
        MediaType::Movie
    } else {
        MediaType::Series
    };

    Ok(MediaDetails {
        id: ProviderMediaId {
            provider: ProviderKind::AnimeXin,
            value: id.to_string(),
        },
        title,
        media_type,
        year: None,
        description: overview,
        tagline: None,
        imdb_rating: rating.map(|r| format!("{r:.1}")),
        director: None,
        stars: None,
        prints: None,
        audios: None,
        poster_url,
        duration: Some("25m".to_string()),
        genres,
        seasons,
        dubs: Vec::new(),
    })
}

pub fn parse_releases(html: &str, season: usize, episode: usize) -> Result<Vec<Release>, AnimeXinError> {
    let document = Html::parse_document(html);
    let select_sel = Selector::parse("select.mirror option, select[name='mirror'] option").map_err(|e| {
        AnimeXinError::Parse(format!("selector error: {e:?}"))
    })?;

    let mut releases = Vec::new();

    for option in document.select(&select_sel) {
        let label = option.text().collect::<String>().trim().to_string();
        let Some(val) = option.value().attr("value") else {
            continue;
        };
        let b64_clean = val.trim();
        if b64_clean.is_empty() || label.to_ascii_lowercase().contains("select video server") {
            continue;
        }

        let decoded_bytes = match base64::engine::general_purpose::STANDARD.decode(b64_clean) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        let snippet = String::from_utf8_lossy(&decoded_bytes);

        if let Some(src) = extract_src(&snippet) {
            let normalized_src = if src.starts_with("//") {
                format!("https:{src}")
            } else {
                src.to_string()
            };

            let language = if label.to_ascii_lowercase().contains("english") || label.to_ascii_lowercase().contains("eng") {
                Some("English".to_string())
            } else if label.to_ascii_lowercase().contains("indonesia") || label.to_ascii_lowercase().contains("indo") {
                Some("Indonesian".to_string())
            } else {
                Some("Multi".to_string())
            };

            let filename = format!("AnimeXin - S{season:02}E{episode:02} - {label}.mp4");

            let mirror = SourceMirror {
                label: label.clone(),
                resolver_url: normalized_src,
                headers: Vec::new(),
                direct_file: false,
            };

            releases.push(Release {
                provider: ProviderKind::AnimeXin,
                filename,
                quality: Some("1080p".to_string()),
                codec: Some("H.264".to_string()),
                language,
                size_bytes: None,
                season: Some(season),
                episode: Some(episode),
                mirrors: vec![mirror],
                resource_id: None,
            });
        }
    }

    // Also check download links for MediaFire or direct mirrors
    let ddl_section_sel = Selector::parse(".soraddlx").ok();
    let h3_sel = Selector::parse(".sorattlx h3").ok();
    let url_block_sel = Selector::parse(".soraurlx").ok();
    let strong_sel = Selector::parse("strong").ok();
    let a_sel = Selector::parse("a[href]").ok();

    if let (Some(sec_sel), Some(h_sel), Some(url_sel), Some(s_sel), Some(link_sel)) =
        (ddl_section_sel, h3_sel, url_block_sel, strong_sel, a_sel)
    {
        for sec in document.select(&sec_sel) {
            let lang_title = sec
                .select(&h_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            for url_block in sec.select(&url_sel) {
                let quality = url_block
                    .select(&s_sel)
                    .next()
                    .map(|el| el.text().collect::<String>().trim().to_string())
                    .unwrap_or_else(|| "1080".to_string());

                for a in url_block.select(&link_sel) {
                    let ddl_label = a.text().collect::<String>().trim().to_string();
                    let Some(href) = a.value().attr("href") else {
                        continue;
                    };
                    if href.is_empty() || href.starts_with('#') {
                        continue;
                    }

                    let full_label = format!("{lang_title} {ddl_label} ({quality}p)");
                    let filename = format!("AnimeXin - S{season:02}E{episode:02} - {full_label}.mp4");

                    let mirror = SourceMirror {
                        label: full_label,
                        resolver_url: href.to_string(),
                        headers: Vec::new(),
                        direct_file: true,
                    };

                    releases.push(Release {
                        provider: ProviderKind::AnimeXin,
                        filename,
                        quality: Some(format!("{quality}p")),
                        codec: Some("H.264".to_string()),
                        language: Some(lang_title.clone()),
                        size_bytes: None,
                        season: Some(season),
                        episode: Some(episode),
                        mirrors: vec![mirror],
                        resource_id: None,
                    });
                }
            }
        }
    }

    Ok(releases)
}

fn extract_src(snippet: &str) -> Option<&str> {
    let lower = snippet.to_ascii_lowercase();
    let idx = lower.find("src=")?;
    let after = &snippet[idx + 4..].trim_start();
    let quote = after.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let after_quote = &after[quote.len_utf8()..];
    let end_idx = after_quote.find(quote)?;
    Some(&after_quote[..end_idx])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_src() {
        let snippet = r#"<iframe src="https://rumble.com/embed/v7d9t4u/?pub=4qfz5q" width="100%"></iframe>"#;
        assert_eq!(
            extract_src(snippet),
            Some("https://rumble.com/embed/v7d9t4u/?pub=4qfz5q")
        );
    }

    #[test]
    fn test_decode_mirror_options() {
        let html = r#"
        <select class="mirror" name="mirror">
            <option value="">Select Video Server</option>
            <option value="PGlmcmFtZSBzcmM9Imh0dHBzOi8vcnVtYmxlLmNvbS9lbWJlZC92N2Q5dDR1LyI+PC9pZnJhbWU+">Hardsub English Rumble AX</option>
        </select>
        "#;
        let releases = parse_releases(html, 1, 158).expect("parse releases");
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].season, Some(1));
        assert_eq!(releases[0].episode, Some(158));
        assert_eq!(releases[0].mirrors[0].label, "Hardsub English Rumble AX");
        assert_eq!(
            releases[0].mirrors[0].resolver_url,
            "https://rumble.com/embed/v7d9t4u/"
        );
    }
}
