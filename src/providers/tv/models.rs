use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub logo: String,
    pub group: String,
    pub stream_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TvCuratedPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub category: &'static str,
    pub url: &'static str,
}

impl TvCuratedPreset {
    pub const ALL: &'static [TvCuratedPreset] = &[
        TvCuratedPreset {
            id: "news",
            name: "Global News",
            description: "World news, headlines & live reports (BBC, CNN, Sky...)",
            category: "News",
            url: "https://iptv-org.github.io/iptv/categories/news.m3u",
        },
        TvCuratedPreset {
            id: "movies",
            name: "Movies & Cinema",
            description: "Classic movies, indie cinema & 24/7 feature films",
            category: "Movies",
            url: "https://iptv-org.github.io/iptv/categories/movies.m3u",
        },
        TvCuratedPreset {
            id: "animation",
            name: "Animation & Kids",
            description: "Cartoons, anime, retro animations & family broadcasts",
            category: "Kids",
            url: "https://iptv-org.github.io/iptv/categories/animation.m3u",
        },
        TvCuratedPreset {
            id: "music",
            name: "Music & Concerts",
            description: "Top charts, music videos, concerts & live broadcasts",
            category: "Music",
            url: "https://iptv-org.github.io/iptv/categories/music.m3u",
        },
        TvCuratedPreset {
            id: "sports",
            name: "Sports Live",
            description: "Motorsports, combat sports, outdoor & athletic leagues",
            category: "Sports",
            url: "https://iptv-org.github.io/iptv/categories/sports.m3u",
        },
        TvCuratedPreset {
            id: "documentary",
            name: "Documentaries",
            description: "Nature, science, history, exploration & tech docs",
            category: "Documentary",
            url: "https://iptv-org.github.io/iptv/categories/documentary.m3u",
        },
        TvCuratedPreset {
            id: "in",
            name: "India (National & Regional)",
            description: "Doordarshan, regional news, entertainment & Indian streams",
            category: "India",
            url: "https://iptv-org.github.io/iptv/countries/in.m3u",
        },
        TvCuratedPreset {
            id: "us",
            name: "United States",
            description: "US national, state & community television broadcasts",
            category: "USA",
            url: "https://iptv-org.github.io/iptv/countries/us.m3u",
        },
        TvCuratedPreset {
            id: "uk",
            name: "United Kingdom",
            description: "UK broadcast, British public channels & culture",
            category: "UK",
            url: "https://iptv-org.github.io/iptv/countries/uk.m3u",
        },
    ];
}

