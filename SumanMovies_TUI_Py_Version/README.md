# 🎬 SumanMovies TUI (Python Edition)

**A modern, async, keyboard-driven Terminal UI for streaming and downloading movies, TV series, anime, and 4K UHD releases — built with Python and Textual.**

---

## ✨ Features

- **Modern Terminal UI**: Built on [Textual](https://textual.textualize.io/) with full mouse & keyboard controls and Catppuccin Mocha styling.
- **Multi-Source Streaming**:
  - **MovieBox**: Global catalog of movies and series with fast HD streams and subtitle auto-fetching.
  - **4KHDHub**: 4K UHD 2160p releases and HubCloud mirrors.
- **Custom Player Branding**: Overrides embedded release group tags and watermarks with **`SumanMovies TUI Api Service • <Movie Title>`** across MPV and VLC.
- **Landing Deck**: Discover Categories, Continue Watching progress tracker, and Favorites.
- **Disk Caching & Settings**: In-app cache clearing, player auto-detection, and persistent watch history.

---

## 🚀 Quick Start

### 1. Requirements:
- Python 3.10+
- `mpv` or `vlc` installed on your machine.

### 2. Installation:

```bash
# Navigate to the Python version directory
cd SumanMovies_TUI_Py_Version

# Create and activate a virtual environment (recommended)
python -m venv venv

# Windows:
.\venv\Scripts\activate

# Linux / macOS:
source venv/bin/activate

# Install dependencies
pip install -r requirements.txt
```

### 3. Running the App:

```bash
python run.py
```

Or install in editable mode:
```bash
pip install -e .
sumanmovies
```

---

## ⌨️ Controls & Shortcuts

| Key | Action |
| :--- | :--- |
| `/` | Focus search bar |
| `Enter` | Select / Open item or stream |
| `Esc` | Back / Clear search |
| `Tab` | Cycle tabs (Discover, Continue Watching, Favorites) |
| `Ctrl+P` | Switch active provider (MovieBox ↔ 4KHDHub) |
| `Ctrl+S` | Open Settings & Preferences |
| `f` | Toggle favorite on details screen |

---

## 📁 Architecture

```
SumanMovies_TUI_Py_Version/
├── pyproject.toml              # Packaging & dependencies
├── requirements.txt            # Pip requirements
├── run.py                      # Direct execution entry point
├── README.md                   # Documentation
└── sumanmovies/
    ├── config.py               # Storage paths & settings
    ├── models.py               # Pydantic data models
    ├── cache.py                # Disk and memory caching
    ├── history.py              # Watch history & favorites
    ├── player.py               # MPV / VLC launcher with custom title
    ├── providers/              # MovieBox & 4KHDHub stream engines
    └── tui/                    # Textual UI app, widgets, screens & TCSS
```
