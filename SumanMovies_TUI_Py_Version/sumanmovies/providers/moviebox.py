"""MovieBox provider implementation for streaming movies and series."""

import re
import json
import base64
import httpx
from typing import List, Optional, Dict, Any
from .base import BaseProvider
from ..models import (
    CatalogItem,
    MediaDetails,
    Season,
    Episode,
    StreamSource,
    SubtitleTrack,
    MediaType,
)
from ..cache import DiskCache

BASE_URL = "https://wefeed.inmov.net"
HEADERS = {
    "User-Agent": "okhttp/4.12.0",
    "Accept": "application/json",
    "Accept-Language": "en",
    "App-Version": "10000",
    "Device-Platform": "android",
}


class MovieBoxProvider(BaseProvider):
    def __init__(self):
        self.cache = DiskCache(namespace="moviebox", ttl_seconds=3600)
        self.client = httpx.AsyncClient(
            headers=HEADERS,
            timeout=15.0,
            follow_redirects=True,
        )

    @property
    def name(self) -> str:
        return "moviebox"

    @property
    def display_name(self) -> str:
        return "MovieBox"

    async def _get_json(self, endpoint: str, params: Optional[Dict[str, Any]] = None) -> Optional[Dict[str, Any]]:
        url = f"{BASE_URL}{endpoint}" if endpoint.startswith("/") else endpoint
        cache_key = f"{url}:{json.dumps(params or {}, sort_keys=True)}"
        cached = self.cache.get(cache_key)
        if cached:
            return cached

        try:
            resp = await self.client.get(url, params=params)
            if resp.status_code == 200:
                data = resp.json()
                self.cache.set(cache_key, data)
                return data
        except Exception:
            pass
        return None

    def _parse_item(self, item: Dict[str, Any]) -> Optional[CatalogItem]:
        item_id = str(item.get("id") or item.get("subjectId") or "")
        if not item_id:
            return None

        title = item.get("title") or item.get("subjectName") or "Unknown"
        media_type_raw = str(item.get("subjectType") or item.get("type") or "1")
        media_type = MediaType.SERIES if media_type_raw in ("2", "series", "tv") else MediaType.MOVIE

        poster = item.get("cover") or item.get("poster") or item.get("img")
        if isinstance(poster, dict):
            poster = poster.get("url") or poster.get("path")

        year_match = re.search(r"\b(19\d\d|20\d\d)\b", str(item.get("releaseDate") or title))
        year = int(year_match.group(1)) if year_match else None

        rating = float(item.get("score") or item.get("rating") or 0.0)
        genres = [g.get("name", "") if isinstance(g, dict) else str(g) for g in item.get("genres", [])]

        return CatalogItem(
            id=item_id,
            provider=self.name,
            title=title,
            year=year,
            media_type=media_type,
            poster_url=poster,
            rating=rating if rating > 0 else None,
            overview=item.get("description") or item.get("intro"),
            genres=[g for g in genres if g],
        )

    async def get_popular(self, page: int = 1) -> List[CatalogItem]:
        data = await self._get_json("/wefeed-mobile-bff/subject-list", {"page": page, "pageSize": 24, "type": "movie"})
        items = []
        if data and "data" in data:
            raw_list = data["data"].get("list") or data["data"].get("items") or []
            for raw in raw_list:
                parsed = self._parse_item(raw)
                if parsed:
                    items.append(parsed)
        return items

    async def get_trending_series(self, page: int = 1) -> List[CatalogItem]:
        data = await self._get_json("/wefeed-mobile-bff/subject-list", {"page": page, "pageSize": 24, "type": "tv"})
        items = []
        if data and "data" in data:
            raw_list = data["data"].get("list") or data["data"].get("items") or []
            for raw in raw_list:
                parsed = self._parse_item(raw)
                if parsed:
                    items.append(parsed)
        return items

    async def get_latest(self, page: int = 1) -> List[CatalogItem]:
        data = await self._get_json("/wefeed-mobile-bff/subject-list", {"page": page, "pageSize": 24, "sort": "latest"})
        items = []
        if data and "data" in data:
            raw_list = data["data"].get("list") or data["data"].get("items") or []
            for raw in raw_list:
                parsed = self._parse_item(raw)
                if parsed:
                    items.append(parsed)
        return items

    async def search(self, query: str, page: int = 1) -> List[CatalogItem]:
        if not query.strip():
            return []
        data = await self._get_json("/wefeed-mobile-bff/search", {"keyword": query, "page": page, "pageSize": 24})
        items = []
        if data and "data" in data:
            raw_list = data["data"].get("list") or data["data"].get("items") or []
            for raw in raw_list:
                parsed = self._parse_item(raw)
                if parsed:
                    items.append(parsed)
        return items

    async def get_details(self, item_id: str) -> Optional[MediaDetails]:
        data = await self._get_json(f"/wefeed-mobile-bff/subject-detail?subjectId={item_id}")
        if not data or "data" not in data:
            return None
        d = data["data"]
        base = self._parse_item(d)
        if not base:
            return None

        seasons: List[Season] = []
        raw_seasons = d.get("seasons") or []
        if raw_seasons:
            for s in raw_seasons:
                s_num = int(s.get("seasonNumber") or s.get("season") or 1)
                episodes = []
                for ep in s.get("episodes") or []:
                    ep_num = int(ep.get("episodeNumber") or ep.get("episode") or 1)
                    episodes.append(
                        Episode(
                            season_number=s_num,
                            episode_number=ep_num,
                            title=ep.get("title") or f"Episode {ep_num}",
                            overview=ep.get("description"),
                            thumbnail=ep.get("cover"),
                        )
                    )
                seasons.append(Season(season_number=s_num, title=f"Season {s_num}", episodes=episodes))

        return MediaDetails(
            id=base.id,
            provider=self.name,
            title=base.title,
            year=base.year,
            media_type=base.media_type,
            poster_url=base.poster_url,
            banner_url=d.get("banner") or base.poster_url,
            overview=base.overview,
            rating=base.rating,
            genres=base.genres,
            seasons=seasons,
        )

    async def get_streams(
        self, item_id: str, season: int = 0, episode: int = 0
    ) -> List[StreamSource]:
        endpoint = f"/wefeed-mobile-bff/subject-stream?subjectId={item_id}&season={season}&episode={episode}"
        data = await self._get_json(endpoint)
        sources: List[StreamSource] = []
        if data and "data" in data:
            stream_list = data["data"].get("streams") or data["data"].get("list") or []
            for s in stream_list:
                url = s.get("url") or s.get("playUrl")
                if url and url.startswith("http"):
                    sources.append(
                        StreamSource(
                            url=url,
                            quality=str(s.get("quality") or s.get("resolution") or "1080p"),
                            source_label=str(s.get("name") or s.get("source") or "Server 1"),
                            headers={"User-Agent": "okhttp/4.12.0", "Referer": "https://wefeed.inmov.net/"},
                        )
                    )
        return sources

    async def get_subtitles(
        self, item_id: str, season: int = 0, episode: int = 0
    ) -> List[SubtitleTrack]:
        endpoint = f"/wefeed-mobile-bff/subject-subtitle?subjectId={item_id}&season={season}&episode={episode}"
        data = await self._get_json(endpoint)
        tracks: List[SubtitleTrack] = []
        if data and "data" in data:
            for sub in data["data"].get("subtitles") or []:
                url = sub.get("url")
                if url:
                    tracks.append(
                        SubtitleTrack(
                            id=str(sub.get("id") or len(tracks)),
                            language=sub.get("language") or "en",
                            label=sub.get("name") or sub.get("label") or "English",
                            url=url,
                        )
                    )
        return tracks
