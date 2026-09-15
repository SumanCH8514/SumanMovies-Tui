"""Base abstract class for all streaming providers."""

from abc import ABC, abstractmethod
from typing import List, Optional
from ..models import CatalogItem, MediaDetails, StreamSource, SubtitleTrack


class BaseProvider(ABC):
    @property
    @abstractmethod
    def name(self) -> str:
        pass

    @property
    @abstractmethod
    def display_name(self) -> str:
        pass

    @abstractmethod
    async def get_popular(self, page: int = 1) -> List[CatalogItem]:
        pass

    @abstractmethod
    async def get_trending_series(self, page: int = 1) -> List[CatalogItem]:
        pass

    @abstractmethod
    async def get_latest(self, page: int = 1) -> List[CatalogItem]:
        pass

    @abstractmethod
    async def search(self, query: str, page: int = 1) -> List[CatalogItem]:
        pass

    @abstractmethod
    async def get_details(self, item_id: str) -> Optional[MediaDetails]:
        pass

    @abstractmethod
    async def get_streams(
        self, item_id: str, season: int = 0, episode: int = 0
    ) -> List[StreamSource]:
        pass

    @abstractmethod
    async def get_subtitles(
        self, item_id: str, season: int = 0, episode: int = 0
    ) -> List[SubtitleTrack]:
        pass
