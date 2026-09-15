pub mod client;
pub mod parser;

pub use client::YouTubeClient;

use crate::providers::models::{CatalogItem, MediaDetails, ProviderError, ProviderKind, Release};
use crate::providers::{Provider, ProviderCapabilities, ReleaseProvider};

impl Provider for YouTubeClient {
    fn id(&self) -> ProviderKind {
        ProviderKind::YouTube
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_search: true,
            supports_pagination: false,
            supports_series: false,
            supports_subtitles: true,
            supports_homepage: false,
        }
    }

    async fn search(&self, query: &str, _page: usize) -> Result<Vec<CatalogItem>, ProviderError> {
        self.search(query).await
    }

    async fn details(&self, id: &str) -> Result<MediaDetails, ProviderError> {
        self.details(id).await
    }
}

impl ReleaseProvider for YouTubeClient {
    async fn episode_streams(
        &self,
        id: &str,
        _season: usize,
        _episode: usize,
    ) -> Result<Vec<Release>, ProviderError> {
        self.releases(id).await
    }
}
