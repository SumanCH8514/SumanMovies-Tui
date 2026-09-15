"""Watch history, resume state, and favorites storage."""

import time
import json
from pathlib import Path
from typing import List, Optional
from .config import get_data_dir
from .models import WatchHistoryItem, CatalogItem


class HistoryManager:
    def __init__(self):
        self.history_file = get_data_dir() / "history.json"
        self.favorites_file = get_data_dir() / "favorites.json"
        self._history: List[WatchHistoryItem] = []
        self._favorites: List[CatalogItem] = []
        self.load()

    def load(self):
        if self.history_file.exists():
            try:
                with open(self.history_file, "r", encoding="utf-8") as f:
                    data = json.load(f)
                    self._history = [WatchHistoryItem(**item) for item in data]
            except Exception:
                self._history = []

        if self.favorites_file.exists():
            try:
                with open(self.favorites_file, "r", encoding="utf-8") as f:
                    data = json.load(f)
                    self._favorites = [CatalogItem(**item) for item in data]
            except Exception:
                self._favorites = []

    def save_history(self):
        try:
            with open(self.history_file, "w", encoding="utf-8") as f:
                json.dump([item.model_dump() for item in self._history], f, indent=2)
        except Exception:
            pass

    def save_favorites(self):
        try:
            with open(self.favorites_file, "w", encoding="utf-8") as f:
                json.dump([item.model_dump() for item in self._favorites], f, indent=2)
        except Exception:
            pass

    def get_history(self) -> List[WatchHistoryItem]:
        return sorted(self._history, key=lambda x: x.last_watched_timestamp, reverse=True)

    def get_continue_watching(self) -> List[WatchHistoryItem]:
        return [item for item in self.get_history() if item.is_in_progress()]

    def record_progress(
        self,
        provider: str,
        subject_id: str,
        title: str,
        season: int,
        episode: int,
        progress_sec: int,
        duration_sec: int,
        poster_url: Optional[str] = None,
    ):
        # Update existing or append
        existing = next(
            (
                item
                for item in self._history
                if item.provider == provider
                and item.subject_id == subject_id
                and item.season == season
                and item.episode == episode
            ),
            None,
        )
        now = int(time.time())
        if existing:
            existing.progress_seconds = progress_sec
            existing.duration_seconds = duration_sec or existing.duration_seconds
            existing.last_watched_timestamp = now
            if poster_url:
                existing.poster_url = poster_url
        else:
            new_item = WatchHistoryItem(
                provider=provider,
                subject_id=subject_id,
                title=title,
                season=season,
                episode=episode,
                progress_seconds=progress_sec,
                duration_seconds=duration_sec,
                last_watched_timestamp=now,
                poster_url=poster_url,
            )
            self._history.insert(0, new_item)

        self._history = self._history[:200]
        self.save_history()

    def get_favorites(self) -> List[CatalogItem]:
        return self._favorites

    def is_favorite(self, item_id: str, provider: str) -> bool:
        return any(f.id == item_id and f.provider == provider for f in self._favorites)

    def toggle_favorite(self, item: CatalogItem) -> bool:
        idx = next(
            (
                i
                for i, f in enumerate(self._favorites)
                if f.id == item.id and f.provider == item.provider
            ),
            None,
        )
        if idx is not None:
            self._favorites.pop(idx)
            self.save_favorites()
            return False
        else:
            self._favorites.insert(0, item)
            self.save_favorites()
            return True
