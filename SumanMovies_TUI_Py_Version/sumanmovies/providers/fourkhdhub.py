"""4KHDHub provider implementation for 4K UHD movies & HubCloud streaming."""

import re
import httpx
from bs4 import BeautifulSoup
from typing import List, Optional
from .base import BaseProvider
from ..models import (
    CatalogItem,
    MediaDetails,
    StreamSource,
    SubtitleTrack,
    MediaType,
)
from ..cache import DiskCache

BASE_URL = "https://4khdhub.dad"
HEADERS = {
    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36",
    "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
}


class FourKHDHubProvider(BaseProvider):
    def __init__(self):
        self.cache = DiskCache(namespace="fourkhdhub", ttl_seconds=3600)
        self.client = httpx.AsyncClient(
            headers=HEADERS,
            timeout=15.0,
            follow_redirects=True,
        )

    @property
    def name(self) -> str:
        return "4khdhub"

    @property
    def display_name(self) -> str:
        return "4KHDHub (4K UHD)"

    async def _fetch_html(self, url: str) -> Optional[str]:
        cached = self.cache.get(url)
        if cached:
            return cached
        try:
            resp = await self.client.get(url)
            if resp.status_code == 200:
                self.cache.set(url, resp.text)
                return resp.text
        except Exception:
            pass
        return None

    def _parse_cards(self, html: str) -> List[CatalogItem]:
        soup = BeautifulSoup(html, "html.parser")
        items = []
        for article in soup.select("article, div.item, div.movie-card"):
            link = article.select_one("a[href]")
            if not link:
                continue
            href = link.get("href", "")
            title_elem = article.select_one("h2, h3, .title, .entry-title") or link
            title = title_elem.get_text(strip=True) if title_elem else "Unknown"
            if not title or not href:
                continue

            img = article.select_one("img[src], img[data-src]")
            poster = img.get("data-src") or img.get("src") if img else None

            year_match = re.search(r"\b(19\d\d|20\d\d)\b", title)
            year = int(year_match.group(1)) if year_match else None

            items.append(
                CatalogItem(
                    id=href,
                    provider=self.name,
                    title=title,
                    year=year,
                    media_type=MediaType.MOVIE,
                    poster_url=poster,
                )
            )
        return items

    async def get_popular(self, page: int = 1) -> List[CatalogItem]:
        url = f"{BASE_URL}/page/{page}/" if page > 1 else f"{BASE_URL}/"
        html = await self._fetch_html(url)
        return self._parse_cards(html) if html else []

    async def get_trending_series(self, page: int = 1) -> List[CatalogItem]:
        url = f"{BASE_URL}/category/web-series/page/{page}/" if page > 1 else f"{BASE_URL}/category/web-series/"
        html = await self._fetch_html(url)
        return self._parse_cards(html) if html else []

    async def get_latest(self, page: int = 1) -> List[CatalogItem]:
        return await self.get_popular(page)

    async def search(self, query: str, page: int = 1) -> List[CatalogItem]:
        if not query.strip():
            return []
        url = f"{BASE_URL}/page/{page}/?s={query}" if page > 1 else f"{BASE_URL}/?s={query}"
        html = await self._fetch_html(url)
        return self._parse_cards(html) if html else []

    async def get_details(self, item_id: str) -> Optional[MediaDetails]:
        html = await self._fetch_html(item_id)
        if not html:
            return None
        soup = BeautifulSoup(html, "html.parser")
        title = soup.select_one("h1.entry-title, h1")
        title_text = title.get_text(strip=True) if title else "4K Release"

        img = soup.select_one("div.entry-content img[src], img.attachment-post-thumbnail")
        poster = img.get("src") if img else None

        desc = soup.select_one("div.entry-content p")
        overview = desc.get_text(strip=True) if desc else None

        return MediaDetails(
            id=item_id,
            provider=self.name,
            title=title_text,
            media_type=MediaType.MOVIE,
            poster_url=poster,
            overview=overview,
        )

    async def get_streams(
        self, item_id: str, season: int = 0, episode: int = 0
    ) -> List[StreamSource]:
        html = await self._fetch_html(item_id)
        sources: List[StreamSource] = []
        if not html:
            return sources

        soup = BeautifulSoup(html, "html.parser")
        for btn in soup.select("a[href*='hubcloud'], a[href*='drive'], a.btn-download"):
            href = btn.get("href")
            label = btn.get_text(strip=True) or "4K UHD Stream"
            if href:
                sources.append(
                    StreamSource(
                        url=href,
                        quality="2160p (4K UHD)",
                        source_label=label,
                        headers={"User-Agent": HEADERS["User-Agent"], "Referer": BASE_URL},
                    )
                )
        return sources

    async def get_subtitles(
        self, item_id: str, season: int = 0, episode: int = 0
    ) -> List[SubtitleTrack]:
        return []
