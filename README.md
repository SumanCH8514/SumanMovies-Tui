<div align="center">

# 🎬 SumanMovies-TUI

**A high-performance, keyboard-driven Terminal UI (TUI) for discovering, streaming, and downloading movies, TV series, anime, 4K UHD releases, and live TV directly from your command line.**

[![NPM Version](https://img.shields.io/npm/v/sumanmovies?style=for-the-badge&color=89b4fa&logo=npm&logoColor=white)](https://www.npmjs.com/package/sumanmovies)
[![Rust Version](https://img.shields.io/badge/Rust-1.80+-f38ba8?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-a6e3a1?style=for-the-badge)](LICENSE)
[![Platform Support](https://img.shields.io/badge/Platforms-Linux%20%7C%20macOS%20%7C%20Windows%20%7C%20Termux-cba6f7?style=for-the-badge)](https://github.com/SumanCH8514/SumanMovies-Tui)
[![GitHub Stars](https://img.shields.io/github/stars/SumanCH8514/SumanMovies-Tui?style=for-the-badge&color=fab387)](https://github.com/SumanCH8514/SumanMovies-Tui/stargazers)

<br/>

[Overview](#-overview) • [Key Features](#-key-features) • [Installation](#-installation) • [Prerequisites](#-prerequisites) • [Controls & Shortcuts](#-controls--shortcuts) • [Providers & Modes](#-providers--content-modes) • [Themes](#-theme-engine) • [Configuration](#-configuration--environment) • [Documentation](#-complete-documentation) • [License](#-license)

---

</div>

## 📖 Overview

**SumanMovies-TUI** eliminates heavy ad-supported streaming websites, tracking cookies, and clunky browser players in favor of a sleek, ultra-responsive terminal environment. Powered by **Rust**, **Ratatui**, and **Tokio**, SumanMovies-TUI aggregates high-speed stream links across multiple providers, displays real-time high-resolution artwork posters, and delegates hardware-accelerated playback directly to local players like `mpv`, `IINA`, and `VLC`.

Whether you want to binge a TV season in 4K HDR, search anime, extract YouTube audio/video streams, browse Asian dramas, or stream live IPTV sports channels — SumanMovies-TUI delivers a fluid, keyboard-centric experience.

```text
 ┌────────────────────────────────────────────────────────────────────────────────────────┐
 │  SUMANMOVIES ─ Popular Trending                                     [Provider: 4KHDHub]│
 ├────────────────────────────────────────────────────────────────────────────────────────┤
 │  [ 1 ] Dune: Part Two (2024)                    2160p UHD | HDR10+ | Dolby Atmos       │
 │  [ 2 ] Oppenheimer (2023)                       1080p BluRay | Multi-Audio | DDP 5.1   │
 │  [ 3 ] Interstellar (2014)                      4K Remux | IMAX Edition | DTS-HD MA    │
 │  [ 4 ] Shogun - Season 1 (2024)                 Complete S01 (Episodes 1-10) [4K]      │
 │  [ 5 ] Arcane - Season 2 (2024)                 Web-DL 1080p | Dual Audio [Eng/Jap]    │
 ├────────────────────────────────────────────────────────────────────────────────────────┤
 │  [Enter] Play  •  [d] Download  •  [Ctrl+P] Switch Provider  •  [/settings] Hub        │
 └────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## ✨ Key Features

- ⚡ **Native Hardware Playback**: Direct streaming delegated to `mpv`, `IINA` (macOS), or `VLC` with zero transcoding overhead and custom stream authentication headers passed securely.
- 🍿 **Multi-Source Scraping & Aggregation**:
  - **MovieBox**: Global movies, TV series, anime, and trending catalogs with DASH multi-resolution streams.
  - **4KHDHub**: High-bitrate 2160p UHD, HDR10+, Dolby Vision, and HubCloud mirror resolvers.
  - **YouTube Engine**: Search and stream YouTube videos and extract audio/video with `yt-dlp`.
  - **Dramachi**: Extensive Korean, Chinese, and Asian drama library with multi-episode parsing.
  - **BDIX Mirrors**: Ultra-fast low-latency local FTP mirrors (CircleFTP & DhakaFlix).
  - **Live IPTV**: Parse and stream live M3U / M3U8 television channel playlists.
  - **Stremio Addons**: Install and browse community Stremio HTTP addon manifests.
- 🖼️ **Terminal Artwork Engine**: Multi-protocol poster rendering supporting **Kitty Graphics**, **Sixel**, **iTerm2**, and true-color **Unicode Halfblock** fallbacks.
- 💬 **Automatic Subtitle Sync**: Automatically queries, downloads, extracts, and synchronizes SRT/VTT subtitles into playback in your preferred languages.
- 📥 **Batch Multi-Segment Downloader**: Download individual episodes or queue entire seasons (`d`) with HTTP range resume support, multi-connection workers, and organized directory structures (`Movies/` & `Series/`).
- 🕒 **Progress Resume & History Deck**: Remembers your exact watch position down to the second. Pick up immediately from your home screen or `/history`.
- 🎨 **9 Curated Themes**: Catppuccin (Mocha, Macchiato, Frappé, Latte), TokyoNight, Nord, Dracula, Gruvbox, and Rosé Pine with automatic terminal brightness detection.
- ⚙️ **Interactive Settings Hub**: In-app modal accessible via `/settings` (`Ctrl+S`) to configure download directories, default media players, enabled providers, and themes on the fly.

---

## 🚀 Installation

### Method 1: Via NPM / NPX (Zero-Setup)

Run instantly without manual compilation or package setup:

```bash
# Run immediately with npx
npx sumanmovies
```

Or install globally as a persistent CLI tool:

```bash
npm install -g sumanmovies
sumanmovies
```

---

### Method 2: Official Standalone Shell Installers (Recommended)

Our installer scripts automatically resolve system architecture, download signed binaries, verify SHA256 cryptographic checksums, configure PATH, and **automatically install prerequisites** (`mpv`, `yt-dlp`, `ffmpeg`).

#### Linux, macOS & Android (Termux):
```bash
curl -fsSL https://raw.githubusercontent.com/SumanCH8514/SumanMovies-Tui/main/install.sh | bash
```

> **Note for Termux users**: SumanMovies fully supports Android via Termux! The script automatically configures `mpv`, `yt-dlp`, `ffmpeg`, and `termux-tools`/`termux-am` to launch external Android video players like VLC or MX Player.

#### Windows (PowerShell):
```powershell
irm https://raw.githubusercontent.com/SumanCH8514/SumanMovies-Tui/main/install.ps1 | iex
```

Once installed, simply run:
```bash
sumanmovies
```

*(The legacy alias `sumanmovies-tui` is also preserved and available).*

---

### Method 3: From Source (Rust Cargo)

If you have Rust and Cargo installed (`1.80+` required):

```bash
# Clone the repository
git clone https://github.com/SumanCH8514/SumanMovies-Tui.git
cd SumanMovies-Tui

# Build optimized release binary
cargo build --release

# Run
./target/release/sumanmovies-tui
```

Or install directly via Cargo:
```bash
cargo install --path .
```

---

## 🛠️ Prerequisites

SumanMovies delegates stream rendering and media extraction to native command-line utilities.

| Dependency | Purpose | Status | Recommended Install Command |
| :--- | :--- | :--- | :--- |
| **`mpv`** | High-performance hardware player | **Recommended** | `brew install mpv` / `sudo apt install mpv` / `winget install io.mpv` |
| **`yt-dlp`** | Stream extraction & DASH resolver | **Recommended** | `sudo apt install yt-dlp` / `brew install yt-dlp` / `winget install yt-dlp` |
| **`ffmpeg`** | Stream & audio muxing engine | **Recommended** | `sudo apt install ffmpeg` / `brew install ffmpeg` / `winget install Gyan.FFmpeg` |
| **`IINA`** | macOS native GUI media player | *Alternative* | `brew install --cask iina` |
| **`VLC`** | Cross-platform media player | *Alternative* | `brew install --cask vlc` / `winget install VideoLAN.VLC` |

> 💡 **Automated Setup**: Running our official `install.sh` or `install.ps1` checks for and installs these prerequisites automatically.

---

## 🎮 Controls & Shortcuts

SumanMovies features intuitive Vim-style and standard navigation keys throughout the entire interface.

### Global & Navigation

| Keybinding | Action |
| :--- | :--- |
| <kbd>↑</kbd> <kbd>↓</kbd> / <kbd>k</kbd> <kbd>j</kbd> | Navigate vertical lists, search results, and menu items |
| <kbd>←</kbd> <kbd>→</kbd> / <kbd>h</kbd> <kbd>l</kbd> | Step horizontal card columns, season lists, and pagination |
| <kbd>Enter</kbd> | Open details / Select stream / Launch playback |
| <kbd>Tab</kbd> / <kbd>Shift</kbd> + <kbd>Tab</kbd> | Switch between tabs, sections, and detail panes |
| <kbd>Esc</kbd> / <kbd>q</kbd> | Return to previous screen / Dismiss modals / Exit |
| <kbd>PgUp</kbd> / <kbd>PgDn</kbd> | Scroll lists by full page |
| <kbd>Home</kbd> / <kbd>End</kbd> | Jump directly to top or bottom of results |
| <kbd>c</kbd> | Clear current search query (on Home screen) |
| <kbd>Ctrl</kbd> + <kbd>U</kbd> | Clear entire text in active input field |

### Media & Playback

| Keybinding | Action |
| :--- | :--- |
| <kbd>Enter</kbd> | Start streaming selected release or episode |
| <kbd>Space</kbd> / <kbd>p</kbd> | Resume playback from saved timestamp |
| <kbd>d</kbd> | Queue selected title/episode for background download |
| <kbd>f</kbd> | Add / remove title from Favorites library |
| <kbd>Ctrl</kbd> + <kbd>P</kbd> | Cycle active provider (*MovieBox → 4KHDHub → YouTube → Dramachi → BDIX*) |
| <kbd>r</kbd> | Refresh catalog results or reload active channel playlist |
| <kbd>Del</kbd> | Delete selected entry from Watch History |

### Modes & Slash Commands

Type `/` anywhere in the search bar to run quick commands:

| Command / Shortcut | Description |
| :--- | :--- |
| <kbd>Ctrl</kbd> + <kbd>S</kbd> / `/settings` | Open interactive Settings Hub (Players, Paths, Themes) |
| <kbd>Ctrl</kbd> + <kbd>T</kbd> / `/tv` | Switch to Live TV / IPTV Channel Streaming Mode |
| `/browse` | Browse curated genres and catalog categories |
| `/history` | Open watch history deck and resume unfinished titles |
| `/favorites` | Open starred and favorited titles library |
| `/config` | Manage IPTV playlist URLs or Stremio addon manifests |
| `/list` | List all channels (in TV mode) |
| `/clear` | Clear current search and reset filters |
| `/exit` | Exit SumanMovies cleanly |

---

## 📡 Providers & Content Modes

```text
┌────────────────────────────────────────────────────────────────────────┐
│                        SumanMovies Ecosystem                           │
├──────────────┬──────────────┬──────────────┬──────────────┬────────────┤
│   MovieBox   │   4KHDHub    │   YouTube    │   Dramachi   │  Live TV   │
│ Global VOD   │ 2160p UHD    │ Video/Audio  │ Asian/KDrama │  IPTV M3U  │
│ Multi-Audio  │ HDR & Atmos  │ yt-dlp Pipe  │ Sub & Dub    │ Low Latency│
└──────────────┴──────────────┴──────────────┴──────────────┴────────────┘
```

1. **MovieBox Mode**:
   - Access global movies, television series, trending charts, and popular releases.
   - Intelligent stream resolution with multi-audio audio track options.
2. **4KHDHub Mode**:
   - Curated high-bitrate UHD releases (2160p, 1080p Remux, IMAX, Dolby Vision, HDR10+).
   - Direct scrapers for HubCloud, GDFlix, and high-speed cloud mirrors.
3. **YouTube Streaming Engine**:
   - Search YouTube directly from your terminal.
   - Stream high-resolution video or extract crystal-clear audio streams (MP3/Opus) on the fly via `yt-dlp`.
4. **Dramachi (Asian & K-Drama)**:
   - Dedicated index for Korean, Japanese, Chinese, and Taiwanese dramas with complete episode listings.
5. **Live IPTV & Television Mode**:
   - Custom M3U / M3U8 playlist management.
   - Grouping by country and genre, channel search, and low-latency playback.
6. **Stremio Addon Extensibility**:
   - Add any Stremio HTTP addon manifest to browse custom catalogs, anime feeds, or community torrent scrapers.

---

## 🎨 Theme Engine

SumanMovies ships with 9 hand-crafted color palettes designed for maximum readability and visual appeal:

| Theme | Type | Description |
| :--- | :--- | :--- |
| **Mocha** *(Default)* | Dark | Rich, warm Catppuccin palette with soothing pastels |
| **TokyoNight** | Dark | Clean, modern neon aesthetics inspired by Tokyo nights |
| **Nord** | Dark | Cool, arctic bluish tones with clean contrast |
| **Dracula** | Dark | Vibrant gothic purples, pinks, and cyans |
| **Gruvbox** | Dark | Retro groove warm colors with soft contrast |
| **RosePine** | Dark | Dreamy, muted pine greens and soft floral rose highlights |
| **Macchiato** | Dark | Medium-contrast Catppuccin variant |
| **Frappé** | Dark | Low-contrast muted Catppuccin variant |
| **Latte** | Light | High-clarity light theme designed for bright environments |

> Cycle themes instantly in-app via `/settings` or set your preferred theme permanently in `config.json`.

---

## ⚙️ Configuration & Environment

Configuration is stored in a clean, human-readable JSON file:

- **Linux / Android**: `~/.config/sumanmovies-tui/config.json`
- **macOS**: `~/Library/Application Support/sumanmovies-tui/config.json`
- **Windows**: `%APPDATA%\SumanMovies-Tui\config.json`

### Example `config.json`
```json
{
  "active_mode": "streaming",
  "active_provider": "MovieBox",
  "active_theme": "Mocha",
  "default_player": "mpv",
  "download_dir": "~/Downloads/SumanMovies",
  "auto_update": true,
  "moviebox_enabled": true,
  "fourkhdhub_enabled": true,
  "youtube_enabled": true,
  "dramachi_enabled": true,
  "tv_enabled": true,
  "addons_enabled": false
}
```

### Environment Variables

| Variable | Description | Example |
| :--- | :--- | :--- |
| `SUMANMOVIES_THEME` | Override theme on launch | `tokyonight`, `nord`, `dracula` |
| `SUMANMOVIES_PLAYER` | Preferred media player | `mpv`, `iina`, `vlc`, `android` |
| `SUMANMOVIES_MPV_PATH` | Path to custom `mpv` binary | `/usr/local/bin/mpv` |
| `SUMANMOVIES_VLC_PATH` | Path to custom `vlc` binary | `C:\Program Files\VideoLAN\VLC\vlc.exe` |
| `SUMANMOVIES_IINA_PATH`| Path to custom `iina-cli` binary | `/usr/local/bin/iina-cli` |
| `SUMANMOVIES_IMAGE_PROTOCOL` | Force terminal image protocol | `kitty`, `sixel`, `iterm2`, `none` |
| `SUMANMOVIES_NO_IMAGE` | Disable image poster queries | `1` or `true` |
| `SUMANMOVIES_LOG` | Logging verbosity | `error`, `warn`, `info`, `debug`, `trace` |

---

## 📂 Architecture & Directory Structure

```text
SumanMovies/
├── npm/                        # NPM distribution wrapper & auto-installer
│   ├── bin/sumanmovies.js      # Global CLI entrypoint
│   └── lib/installer.js        # Multi-platform installer & prerequisite resolver
├── src/
│   ├── main.rs                 # CLI entrypoint, panic hook, & terminal initialization
│   ├── config.rs               # JSON configuration schema & file I/O
│   ├── player.rs               # Media player lifecycle & argument mapper
│   ├── download.rs             # Multi-connection chunked HTTP download manager
│   ├── history.rs              # Watch progress tracker & timestamp persistence
│   ├── favorites.rs            # Starred library state management
│   ├── proxy.rs                # Built-in streaming sidecar proxy for VLC/headers
│   ├── net.rs                  # Async HTTP client pool & TLS configuration
│   ├── providers/              # Provider implementations & scrapers
│   │   ├── moviebox/           # MovieBox API, search, & DASH stream decoders
│   │   ├── fourkhdhub/         # 4KHDHub & HubCloud 4K mirror resolvers
│   │   ├── youtube/            # YouTube search & yt-dlp playback integration
│   │   ├── dramachi/           # Asian & K-Drama catalog scraper
│   │   ├── bdix/               # Low-latency BDIX FTP mirrors
│   │   ├── tv/                 # M3U / M3U8 Live IPTV channel parser
│   │   └── addons/             # Stremio HTTP community addon engine
│   └── tui/                    # Ratatui user interface
│       ├── app/                # Application state machine & event loop
│       ├── screens/            # Home, Details, Settings, and Help views
│       ├── widgets/            # Custom poster renderer, modals, badges, scrollbars
│       └── theme.rs            # 9-palette curated theme engine
├── tests/                      # Live integration & playback verification suite
├── install.sh                  # One-line installer for Linux, macOS, & Termux
├── install.ps1                 # One-line installer for Windows PowerShell
└── Cargo.toml                  # Rust dependencies & crate metadata
```

---

## 📚 Complete Documentation

SumanMovies-TUI includes a comprehensive **mdBook** documentation suite located in the [`docs/`](docs/) directory. Below is the complete index of technical guides and architecture references:

### 🚀 Getting Started & Configuration

| Guide | Description |
| :--- | :--- |
| 📖 [Documentation Overview](docs/README.md) | Introduction, prerequisite quick install matrix, and document map |
| 📦 [Installation Guide](docs/installation.md) | Step-by-step package setup, pre-built binary verification, and paths |
| 🎮 [Controls & Navigation](docs/controls.md) | Complete keybindings, Vim movement, search filtering, and slash commands |
| ⚙️ [Configuration Guide](docs/config.md) | `config.json` schema, Settings Hub (`Ctrl+S`), and environment variables |

### 🍿 Features, Modes & Providers

| Guide | Description |
| :--- | :--- |
| 🌐 [Content Providers](docs/providers.md) | MovieBox, 4KHDHub, YouTube, Dramachi, and BDIX mirror scrapers |
| ⚡ [Hardware Media Players](docs/players.md) | `mpv`, `IINA`, and `VLC` flag mappings, sidecar proxy, and tracking |
| 📥 [Batch Downloader](docs/downloads.md) | Multi-connection chunked downloads, HTTP range resume, and folder structure |
| 📺 [Live TV & IPTV](docs/tv-mode.md) | M3U / M3U8 playlist manager, channel parsing, and live streaming |
| 🔌 [Stremio Addons](docs/addons-mode.md) | Installing community HTTP addon manifests, catalogs, and streams |

### 🏗️ Architecture & Internals

| Guide | Description |
| :--- | :--- |
| 🏛️ [System Architecture](docs/architecture.md) | Subsystem diagrams, Tokio async event loop, and task cancellation |
| 🧩 [Module Breakdown](docs/modules.md) | Crate topology, module responsibilities, and call boundaries |
| 💾 [Caching Strategy](docs/cache.md) | Binary disk caching, TTL policies, and LRU memory management |
| 📝 [Logging System](docs/logging.md) | File logging, rotation, panic backtraces, and tracing diagnostics |
| 🌐 [Cross-Platform Operations](docs/cross-platform.md) | Platform compatibility matrix across Linux, macOS, Windows, and Termux |

### 🛠️ Reference, QA & Maintenance

| Guide | Description |
| :--- | :--- |
| 🧪 [Testing Suite](docs/testing.md) | Unit tests, live provider integration tests, and verification gates |
| 🔍 [Debugging Guide](docs/debugging.md) | Troubleshooting playback errors, terminal graphics glitches, and proxies |
| ⚠️ [Known Issues](docs/known-issues.md) | Tracked terminal quirks, hardware acceleration workarounds, and caveats |
| 📋 [Release Checklist](docs/release-checklist.md) | Multi-target cross-compilation, checksum signing, and release workflow |
| 🤝 [Contributing Guidelines](CONTRIBUTING.md) | Development workflow, coding style, and pull request procedures |
| 📜 [Changelog](CHANGELOG.md) | Detailed version history, breaking changes, and roadmap updates |

> 💡 **View Documentation Locally**: You can build and view the full interactive documentation book in your browser by running:
> ```bash
> mdbook serve
> ```

---

## 📄 License

This project is licensed under the [MIT License](LICENSE) — free to use, modify, and distribute.

---

## 👤 Author & Support

Crafted with ❤️ by **Suman**.

- **GitHub**: [@SumanCH8514](https://github.com/SumanCH8514)
- **Repository**: [SumanMovies-Tui](https://github.com/SumanCH8514/SumanMovies-Tui)
- **NPM Package**: [sumanmovies](https://www.npmjs.com/package/sumanmovies)

If you enjoy using SumanMovies-TUI, consider giving it a ⭐ on [GitHub](https://github.com/SumanCH8514/SumanMovies-Tui)!
