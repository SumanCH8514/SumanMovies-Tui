mod client;
pub mod parser;

pub use client::{AnimeXinClient, AnimeXinError};

use crate::providers::models::{CatalogItem, MediaDetails, ProviderError, ProviderKind, Release};
use crate::providers::{Provider, ProviderCapabilities, ReleaseProvider};

impl From<AnimeXinError> for ProviderError {
    fn from(err: AnimeXinError) -> Self {
        match err {
            AnimeXinError::Network(e) => ProviderError::Network(e.to_string()),
            AnimeXinError::InvalidUrl(u) => ProviderError::Parsing(format!("Invalid URL: {u}")),
            AnimeXinError::Parse(p) => ProviderError::Parsing(p),
            AnimeXinError::NoPlayableMirror(msg) => ProviderError::Unavailable(msg),
        }
    }
}

impl Provider for AnimeXinClient {
    fn id(&self) -> ProviderKind {
        ProviderKind::AnimeXin
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_search: true,
            supports_pagination: true,
            supports_series: true,
            supports_subtitles: true,
            supports_homepage: false,
        }
    }

    async fn search(&self, query: &str, page: usize) -> Result<Vec<CatalogItem>, ProviderError> {
        self.search(query, page).await.map_err(ProviderError::from)
    }

    async fn details(&self, id: &str) -> Result<MediaDetails, ProviderError> {
        self.details(id).await.map_err(ProviderError::from)
    }
}

impl ReleaseProvider for AnimeXinClient {
    async fn episode_streams(
        &self,
        id: &str,
        season: usize,
        episode: usize,
    ) -> Result<Vec<Release>, ProviderError> {
        self.releases(id, season, episode)
            .await
            .map_err(ProviderError::from)
    }
}
