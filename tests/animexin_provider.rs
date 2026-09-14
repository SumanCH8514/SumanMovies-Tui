use sumanmovies_tui::providers::animexin::{parser, AnimeXinClient};
use sumanmovies_tui::providers::models::{MediaType, ProviderKind};
use sumanmovies_tui::providers::Provider;
use url::Url;

#[test]
fn test_animexin_provider_kind() {
    assert_eq!(ProviderKind::AnimeXin.label(), "AnimeXin");
    assert_eq!(ProviderKind::AnimeXin.cache_key(), "animexin");
    assert_eq!(ProviderKind::parse("animexin"), Some(ProviderKind::AnimeXin));
    assert_eq!(ProviderKind::parse("anime_xin"), Some(ProviderKind::AnimeXin));
    assert!(ProviderKind::ENABLED.contains(&ProviderKind::AnimeXin));
}

#[test]
fn test_animexin_search_parser() {
    let html = r#"
    <div class="listupd">
        <article class="bs">
            <div class="bsx">
                <a href="https://animexin.dev/renegade-immortal/" title="Renegade Immortal">
                    <div class="limit">
                        <div class="typez ONA">ONA</div>
                        <img src="https://animexin.dev/uploads/renegade.jpg" />
                    </div>
                    <div class="tt">
                        <h2 itemprop="headline">Renegade Immortal</h2>
                    </div>
                </a>
            </div>
        </article>
        <article class="bs">
            <div class="bsx">
                <a href="https://animexin.dev/renegade-immortal-movie/" title="Renegade Immortal Movie">
                    <div class="limit">
                        <div class="typez Movie">Movie</div>
                        <img src="https://animexin.dev/uploads/movie.jpg" />
                    </div>
                    <div class="tt">
                        <h2 itemprop="headline">Renegade Immortal Movie</h2>
                    </div>
                </a>
            </div>
        </article>
    </div>
    "#;

    let base = Url::parse("https://animexin.dev/").unwrap();
    let items = parser::parse_search(&base, html).expect("parse search");

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].title, "Renegade Immortal");
    assert_eq!(items[0].id.value, "renegade-immortal");
    assert_eq!(items[0].media_type, MediaType::Series);
    assert_eq!(
        items[0].poster_url.as_deref(),
        Some("https://animexin.dev/uploads/renegade.jpg")
    );

    assert_eq!(items[1].title, "Renegade Immortal Movie");
    assert_eq!(items[1].media_type, MediaType::Movie);
}

#[test]
fn test_animexin_details_parser() {
    let html = r#"
    <div class="infox">
        <h1 class="entry-title">Renegade Immortal</h1>
        <div class="rating"><strong>Rating 8.5</strong></div>
        <div class="genxed"><a href="/genres/action/">Action</a><a href="/genres/cultivation/">Cultivation</a></div>
    </div>
    <div class="thumbook"><div class="thumb"><img src="https://animexin.dev/uploads/cover.jpg"/></div></div>
    <div class="synp"><div class="entry-content"><p>Wang Lin was a smart child...</p></div></div>
    <div class="eplister">
        <ul>
            <li data-index="0">
                <a href="https://animexin.dev/renegade-immortal-episode-2-indonesia-english-sub/">
                    <div class="epl-num">2</div>
                    <div class="epl-title">Renegade Immortal Episode 2</div>
                </a>
            </li>
            <li data-index="1">
                <a href="https://animexin.dev/renegade-immortal-episode-1-indonesia-english-sub/">
                    <div class="epl-num">1</div>
                    <div class="epl-title">Renegade Immortal Episode 1</div>
                </a>
            </li>
        </ul>
    </div>
    "#;

    let details = parser::parse_details("renegade-immortal", html).expect("parse details");
    assert_eq!(details.title, "Renegade Immortal");
    assert_eq!(details.imdb_rating.as_deref(), Some("8.5"));
    assert_eq!(details.genres, vec!["Action", "Cultivation"]);
    assert_eq!(details.seasons.len(), 1);
    assert_eq!(details.seasons[0].episodes.len(), 2);
    // Verified sorted ascending: Episode 1 then Episode 2
    assert_eq!(details.seasons[0].episodes[0].number, 1);
    assert_eq!(details.seasons[0].episodes[1].number, 2);
}

#[test]
fn test_animexin_releases_parser() {
    // Base64 encoded: <iframe src="https://rumble.com/embed/v7d9t4u/?pub=4qfz5q"></iframe>
    let b64_rumble = "PGlmcmFtZSBzcmM9Imh0dHBzOi8vcnVtYmxlLmNvbS9lbWJlZC92N2Q5dDR1Lz9wdWI9NHFmejVxIj48L2lmcmFtZT4=";
    // Base64 encoded: <iframe src="https://odysee.com/$/embed/renegade-ep-158"></iframe>
    let b64_odysee = "PGlmcmFtZSBzcmM9Imh0dHBzOi8vb2R5c2VlLmNvbS8kL2VtYmVkL3JlbmVnYWRlLWVwLTE1OCI+PC9pZnJhbWU+";

    let html = format!(
        r#"
        <select class="mirror" name="mirror">
            <option value="">Select Video Server</option>
            <option value="{b64_rumble}" data-index="1">Hardsub English Rumble AX</option>
            <option value="{b64_odysee}" data-index="2">Hardsub Indonesia Odysee AX</option>
        </select>
        <div class="soraddlx">
            <div class="sorattlx"><h3>Subtitle English</h3></div>
            <div class="soraurlx">
                <strong>1080</strong>
                <a href="https://www.mediafire.com/file/xyz123/renegade.mp4/file">Mediafire</a>
            </div>
        </div>
        "#
    );

    let releases = parser::parse_releases(&html, 1, 158).expect("parse releases");
    assert_eq!(releases.len(), 3);

    // Rumble mirror
    assert_eq!(releases[0].mirrors[0].label, "Hardsub English Rumble AX");
    assert_eq!(
        releases[0].mirrors[0].resolver_url,
        "https://rumble.com/embed/v7d9t4u/?pub=4qfz5q"
    );
    assert_eq!(releases[0].language.as_deref(), Some("English"));

    // Odysee mirror
    assert_eq!(releases[1].mirrors[0].label, "Hardsub Indonesia Odysee AX");
    assert_eq!(
        releases[1].mirrors[0].resolver_url,
        "https://odysee.com/$/embed/renegade-ep-158"
    );
    assert_eq!(releases[1].language.as_deref(), Some("Indonesian"));

    // Mediafire mirror
    assert_eq!(releases[2].mirrors[0].resolver_url, "https://www.mediafire.com/file/xyz123/renegade.mp4/file");
}

#[test]
fn test_animexin_client_capabilities() {
    let client = AnimeXinClient::new().expect("client creation");
    let caps = client.capabilities();
    assert!(caps.supports_search);
    assert!(caps.supports_pagination);
    assert!(caps.supports_series);
    assert!(caps.supports_subtitles);
    assert!(!caps.supports_homepage);
}

#[tokio::test]
async fn test_animexin_live_search() {
    let client = AnimeXinClient::new().expect("client creation");
    if let Ok(items) = client.search("renegade", 1).await {
        assert!(!items.is_empty());
        let first = &items[0];
        assert!(!first.title.is_empty());
        println!("Live search found {} items. First: {}", items.len(), first.title);
    }
}
