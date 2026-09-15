"""Data models for SumanMovies TUI Python version."""

from enum import Enum
from typing import Optional, List, Dict
from pydantic import BaseModel, Field


class MediaType(str, Enum):
    MOVIE = "movie"
    SERIES = "series"
    ANIME = "anime"
    LIVE = "live"


class SubtitleTrack(BaseModel):
    id: str
    language: str
    label: str
    url: str
    format: str = "srt"


class StreamSource(BaseModel):
    url: str
    quality: str = "auto"
    source_label: str = "Direct"
    headers: Dict[str, str] = Field(default_factory=dict)
    subtitles: List[SubtitleTrack] = Field(default_factory=list)


class Episode(BaseModel):
    season_number: int
    episode_number: int
    title: str = ""
    overview: Optional[str] = None
    thumbnail: Optional[str] = None
    duration_seconds: Optional[int] = None


class Season(BaseModel):
    season_number: int
    title: str = ""
    episodes: List[Episode] = Field(default_factory=list)


class CatalogItem(BaseModel):
    id: str
    provider: str
    title: str
    year: Optional[int] = None
    media_type: MediaType = MediaType.MOVIE
    poster_url: Optional[str] = None
    banner_url: Optional[str] = None
    rating: Optional[float] = None
    overview: Optional[str] = None
    genres: List[str] = Field(default_factory=list)


class MediaDetails(BaseModel):
    id: str
    provider: str
    title: str
    year: Optional[int] = None
    media_type: MediaType = MediaType.MOVIE
    poster_url: Optional[str] = None
    banner_url: Optional[str] = None
    overview: Optional[str] = None
    rating: Optional[float] = None
    duration_minutes: Optional[int] = None
    genres: List[str] = Field(default_factory=list)
    seasons: List[Season] = Field(default_factory=list)


class WatchHistoryItem(BaseModel):
    provider: str
    subject_id: str
    title: str
    season: int = 0
    episode: int = 0
    progress_seconds: int = 0
    duration_seconds: int = 0
    last_watched_timestamp: int = 0
    poster_url: Optional[str] = None

    def is_in_progress(self) -> bool:
        if self.duration_seconds <= 0:
            return self.progress_seconds > 10
        pct = (self.progress_seconds / self.duration_seconds) * 100
        return 2.0 <= pct <= 92.0
