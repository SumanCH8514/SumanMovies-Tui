"""Configuration and storage paths for SumanMovies TUI."""

import os
import json
from pathlib import Path
from typing import Dict, Any
import platformdirs

APP_NAME = "SumanMovies-TUI"
APP_AUTHOR = "Suman"


def get_data_dir() -> Path:
    path = Path(platformdirs.user_data_dir(APP_NAME, APP_AUTHOR))
    path.mkdir(parents=True, exist_ok=True)
    return path


def get_cache_dir() -> Path:
    path = Path(platformdirs.user_cache_dir(APP_NAME, APP_AUTHOR))
    path.mkdir(parents=True, exist_ok=True)
    return path


def get_downloads_dir() -> Path:
    home = Path.home()
    default_dir = home / "Downloads" / "SumanMovies"
    default_dir.mkdir(parents=True, exist_ok=True)
    return default_dir


class Settings:
    DEFAULT_CONFIG = {
        "player": "auto",  # 'mpv', 'vlc', 'iina', 'auto'
        "theme": "catppuccin_mocha",
        "downloads_path": str(get_downloads_dir()),
        "preferred_language": "en",
        "auto_sync_subtitles": True,
        "content_mode": "streaming",  # 'streaming', 'live_tv', 'addons'
        "default_provider": "moviebox",
    }

    def __init__(self):
        self.config_file = get_data_dir() / "settings.json"
        self._data: Dict[str, Any] = self.DEFAULT_CONFIG.copy()
        self.load()

    def load(self):
        if self.config_file.exists():
            try:
                with open(self.config_file, "r", encoding="utf-8") as f:
                    loaded = json.load(f)
                    self._data.update(loaded)
            except Exception:
                pass

    def save(self):
        try:
            with open(self.config_file, "w", encoding="utf-8") as f:
                json.dump(self._data, f, indent=2)
        except Exception:
            pass

    def get(self, key: str, default: Any = None) -> Any:
        return self._data.get(key, default)

    def set(self, key: str, value: Any):
        self._data[key] = value
        self.save()
