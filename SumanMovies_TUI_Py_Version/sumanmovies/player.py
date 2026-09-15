"""Media player process launcher (MPV, VLC, IINA) with custom title branding."""

import os
import sys
import shutil
import asyncio
import subprocess
from typing import Optional, Dict, List


def format_media_title(title: Optional[str]) -> str:
    """Format custom player title to override container and torrent watermarks."""
    if title and title.strip():
        return f"SumanMovies TUI Api Service • {title.strip()}"
    return "SumanMovies TUI Api Service"


def detect_players() -> List[str]:
    """Detect available media players in PATH and standard installation paths."""
    players = []
    if shutil.which("mpv") or (sys.platform == "win32" and shutil.which("mpv.exe")):
        players.append("mpv")

    if shutil.which("vlc") or (sys.platform == "win32" and shutil.which("vlc.exe")):
        players.append("vlc")

    if sys.platform == "darwin" and os.path.exists("/Applications/IINA.app"):
        players.append("iina")

    # Windows fallback search
    if sys.platform == "win32" and "vlc" not in players:
        for vlc_path in [
            r"C:\Program Files\VideoLAN\VLC\vlc.exe",
            r"C:\Program Files (x86)\VideoLAN\VLC\vlc.exe",
        ]:
            if os.path.exists(vlc_path):
                players.append("vlc")
                break

    return players


def build_mpv_command(
    url: str,
    title: Optional[str] = None,
    headers: Optional[Dict[str, str]] = None,
    subtitle_url: Optional[str] = None,
    resume_seconds: Optional[int] = None,
) -> List[str]:
    executable = shutil.which("mpv") or ("mpv.exe" if sys.platform == "win32" else "mpv")
    media_title = format_media_title(title)
    cmd = [
        executable,
        f"--force-media-title={media_title}",
        f"--title={media_title}",
        "--geometry=50%:50%",
        "--idle=no",
        "--keep-open=no",
    ]

    if resume_seconds and resume_seconds > 0:
        cmd.append(f"--start={resume_seconds}")

    if subtitle_url:
        cmd.append(f"--sub-file={subtitle_url}")

    if headers:
        for k, v in headers.items():
            if k.lower() == "user-agent":
                cmd.append(f"--user-agent={v}")
            elif k.lower() == "referer":
                cmd.append(f"--referrer={v}")
        fields = [
            f"{k}: {v}"
            for k, v in headers.items()
            if k.lower() not in ("user-agent", "referer")
        ]
        if fields:
            cmd.append(f"--http-header-fields={','.join(fields)}")

    cmd.append(url)
    return cmd


def build_vlc_command(
    url: str,
    title: Optional[str] = None,
    headers: Optional[Dict[str, str]] = None,
    subtitle_url: Optional[str] = None,
    resume_seconds: Optional[int] = None,
) -> List[str]:
    executable = shutil.which("vlc") or ("vlc.exe" if sys.platform == "win32" else "vlc")
    media_title = format_media_title(title)
    cmd = [
        executable,
        f"--meta-title={media_title}",
        "--play-and-exit",
    ]

    if resume_seconds and resume_seconds > 0:
        cmd.append(f"--start-time={resume_seconds}")

    if subtitle_url:
        cmd.append(f"--sub-file={subtitle_url}")

    if headers:
        for k, v in headers.items():
            if k.lower() == "referer":
                cmd.append(f"--http-referrer={v}")
            elif k.lower() == "user-agent":
                cmd.append(f"--http-user-agent={v}")

    cmd.append(url)
    return cmd


async def launch_player(
    url: str,
    player_preference: str = "auto",
    title: Optional[str] = None,
    headers: Optional[Dict[str, str]] = None,
    subtitle_url: Optional[str] = None,
    resume_seconds: Optional[int] = None,
) -> asyncio.subprocess.Process:
    detected = detect_players()
    chosen = "mpv"
    if player_preference != "auto" and player_preference in detected:
        chosen = player_preference
    elif "mpv" in detected:
        chosen = "mpv"
    elif "vlc" in detected:
        chosen = "vlc"
    elif "iina" in detected:
        chosen = "iina"

    if chosen == "vlc":
        args = build_vlc_command(url, title, headers, subtitle_url, resume_seconds)
    else:
        args = build_mpv_command(url, title, headers, subtitle_url, resume_seconds)

    proc = await asyncio.create_subprocess_exec(
        *args,
        stdin=asyncio.subprocess.DEVNULL,
        stdout=asyncio.subprocess.DEVNULL,
        stderr=asyncio.subprocess.PIPE,
    )
    return proc
