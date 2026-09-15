"""Disk and in-memory cache system for SumanMovies TUI."""

import os
import time
import json
import hashlib
import shutil
from pathlib import Path
from typing import Optional, Any
from .config import get_cache_dir


def _hash_key(key: str) -> str:
    return hashlib.md5(key.encode("utf-8")).hexdigest()


class DiskCache:
    def __init__(self, namespace: str = "general", ttl_seconds: int = 86400):
        self.dir = get_cache_dir() / namespace
        self.dir.mkdir(parents=True, exist_ok=True)
        self.ttl = ttl_seconds

    def get(self, key: str) -> Optional[Any]:
        hashed = _hash_key(key)
        file_path = self.dir / f"{hashed}.json"
        if not file_path.exists():
            return None
        try:
            with open(file_path, "r", encoding="utf-8") as f:
                payload = json.load(f)
            if time.time() > payload.get("expires_at", 0):
                file_path.unlink(missing_ok=True)
                return None
            return payload.get("data")
        except Exception:
            return None

    def set(self, key: str, data: Any, ttl_seconds: Optional[int] = None):
        hashed = _hash_key(key)
        file_path = self.dir / f"{hashed}.json"
        expiry = time.time() + (ttl_seconds or self.ttl)
        payload = {"expires_at": expiry, "data": data}
        try:
            temp_path = self.dir / f"{hashed}.tmp"
            with open(temp_path, "w", encoding="utf-8") as f:
                json.dump(payload, f)
            temp_path.replace(file_path)
        except Exception:
            pass


def clear_all_cache() -> int:
    """Purge all temporary disk cache files and return deleted file count."""
    cache_root = get_cache_dir()
    count = 0
    if cache_root.exists():
        for item in cache_root.glob("**/*"):
            if item.is_file():
                try:
                    item.unlink(missing_ok=True)
                    count += 1
                except Exception:
                    pass
    return count
