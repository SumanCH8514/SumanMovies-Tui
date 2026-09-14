<div align="center">

# 🎬 SumanMovies

**A blazing-fast, keyboard-driven Terminal UI for streaming and downloading movies, TV series, anime, and 4K UHD releases directly from your command line.**

[![NPM Version](https://img.shields.io/npm/v/sumanmovies?style=for-the-badge&color=cbA6f7&logo=npm)](https://www.npmjs.com/package/sumanmovies)
[![Rust](https://img.shields.io/badge/Rust-1.80+-orange?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-green?style=for-the-badge)](https://github.com/SumanCH8514)

[Quick Start](#-installation) • [Features](#-features) • [Tech Stack](#-tech-stack) • [Controls](#-controls--usage) • [License](#-license)

---

</div>

## 📖 Overview

**SumanMovies** replaces ad-heavy web streaming portals and slow browser players with a lightweight, distraction-free terminal experience. It scrapes and aggregates high-speed stream links across multiple providers, displaying real-time artwork previews and delegating hardware-accelerated playback directly to `mpv`, `IINA`, or `VLC`.

---

## ✨ Features

- 🚀 **Zero-Config Streaming**: Instantly watch content without ads, popups, or browser clutter.
- 🍿 **Multi-Provider Aggregator**: Switch seamlessly between **MovieBox**, **4KHDHub** (2160p UHD / HDR / Dolby Vision releases), BDIX mirrors, and custom Community Addons.
- 🖼️ **Concurrent Artwork Engine**: High-performance asynchronous poster rendering with Kitty Graphics, Sixel, and true-color Unicode Halfblock fallbacks.
- ⚡ **Native Hardware Acceleration**: Direct playback delegated to `mpv` with automatic stream headers, multi-audio language selection (Hindi, English, etc.), and subtitle synchronization.
- 📥 **Batch Downloads**: Download individual episodes or entire seasons with HTTP range resume support (`d`).
- 🕒 **Watch History & Progress Resume**: Automatically saves playback state and timestamps so you can resume exactly where you left off.
- 🎨 **Rich Theme Engine**: Curated palettes including Catppuccin (Mocha, Macchiato, Frappé, Latte), TokyoNight, Nord, Dracula, and High-Contrast modes.

---

## 🛠️ Tech Stack

<div align="left">

| Component | Technology / Library |
| :--- | :--- |
| **Core Language** | ![Rust](https://img.shields.io/badge/Rust-000000?style=flat-square&logo=rust&logoColor=white) |
| **Terminal UI Framework** | ![Ratatui](https://img.shields.io/badge/Ratatui-TUI_Engine-blueviolet?style=flat-square) `crossterm` |
| **Async Runtime & Networking** | ![Tokio](https://img.shields.io/badge/Tokio-Async_I%2FO-red?style=flat-square) `reqwest` |
| **Media Player Integration** | ![MPV](https://img.shields.io/badge/MPV-Media_Player-purple?style=flat-square) `libavformat` |
| **NPM CLI Distribution** | ![Node.js](https://img.shields.io/badge/Node.js-NPM_Wrapper-339933?style=flat-square&logo=node.js&logoColor=white) |

</div>

---

## 🚀 Installation

### Option 1: Via NPM / NPX (Recommended)

No manual build tools or dependencies required. Auto-configures both `sumanmovies` and `mpv`:

```bash
# Run instantly without installing
npx sumanmovies
```

```bash
# Or install globally to run anywhere
npm install -g sumanmovies
sumanmovies
```

---

### Option 2: Standalone Shell Installers

#### Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/SumanCH8514/SumanMovies-Tui/main/install.ps1 | iex
```

#### macOS & Linux (Bash / cURL)
```bash
curl -fsSL https://raw.githubusercontent.com/SumanCH8514/SumanMovies-Tui/main/install.sh | bash
```

---

### Option 3: From Source (Rust Cargo)

```bash
# Clone the repository
git clone https://github.com/SumanCH8514/SumanMovies-Tui.git
cd SumanMovies-Tui

# Build optimized release binary
cargo build --release

# Run
./target/release/moviebox-tui
```

---

## 🎮 Controls & Usage

| Keybinding | Action |
| :--- | :--- |
| <kbd>↑</kbd> <kbd>↓</kbd> <kbd>←</kbd> <kbd>→</kbd> / <kbd>h</kbd> <kbd>j</kbd> <kbd>k</kbd> <kbd>l</kbd> | Navigate items & grid |
| <kbd>Enter</kbd> | Select item / Launch playback |
| <kbd>Tab</kbd> | Switch Tabs (Continue Watching ↔ Favorites) |
| <kbd>Ctrl</kbd> + <kbd>P</kbd> | Switch Provider (**MovieBox** / **4KHDHub** / **BDIX**) |
| <kbd>Ctrl</kbd> + <kbd>S</kbd> | Open Settings Hub (Player, Download Path, Theme) |
| <kbd>Ctrl</kbd> + <kbd>T</kbd> | Cycle Visual Theme |
| <kbd>d</kbd> | Download selected release / episode |
| <kbd>f</kbd> | Toggle Favorite |
| <kbd>Esc</kbd> / <kbd>q</kbd> | Back / Quit |

---

## 📂 Project Structure

```
SumanMovies/
├── npm/                        # NPM distribution wrapper & auto-installer
│   ├── bin/sumanmovies.js      # CLI entrypoint
│   └── lib/installer.js        # Multiplatform prerequisite resolver
├── src/
│   ├── main.rs                 # Application entrypoint
│   ├── player.rs               # Media player lifecycle & argument mapper
│   ├── providers/              # Streaming provider engines
│   │   ├── moviebox/           # MovieBox API scraper & DASH resolver
│   │   └── fourkhdhub/         # 4KHDHub & HubCloud 4K mirror decoders
│   └── tui/                    # Ratatui user interface
│       ├── app/                # Application state machine & event loop
│       ├── screens/            # Home, Details, Settings, and Search views
│       └── widgets/            # Posters, badges, and scrollbars
├── tests/                      # Live integration & playback verification suite
└── Cargo.toml                  # Rust dependencies & metadata
```

---

## 📄 License

This project is open-source software licensed under the [MIT License](LICENSE).

---

## 👤 Author & Contact

**Suman**
- GitHub: [@SumanCH8514](https://github.com/SumanCH8514)
- NPM Package: [sumanmovies](https://www.npmjs.com/package/sumanmovies)
