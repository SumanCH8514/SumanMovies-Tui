<div align="center">

# 🎬 SumanMovies

**A blazing-fast, keyboard-driven Terminal UI for streaming and downloading movies, TV series, anime, and 4K UHD releases directly from your command line.**

[![NPM Version](https://img.shields.io/npm/v/sumanmovies?style=for-the-badge&color=cbA6f7&logo=npm)](https://www.npmjs.com/package/sumanmovies)
[![Rust](https://img.shields.io/badge/Rust-1.80+-orange?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)](https://github.com/SumanCH8514)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-green?style=for-the-badge)](https://github.com/SumanCH8514)

</div>

---

## 🚀 Quick Start

Run directly with zero installation:
```bash
npx sumanmovies
```

Or install globally to use anywhere:
```bash
npm install -g sumanmovies
sumanmovies
```

---

## ✨ Features

- 🍿 **Multi-Provider Aggregator**: Stream seamlessly from **MovieBox**, **4KHDHub** (2160p UHD HDR/DV), BDIX, and community addons.
- 🖼️ **Terminal Artwork Engine**: Concurrent asynchronous poster rendering across Kitty, Sixel, and true-color Unicode terminals.
- ⚡ **Auto-Configured Media Player**: Integrates with `mpv` player out of the box with audio track selection, subtitles, and hardware acceleration.
- 🕒 **Watch History & Resume**: Automatically preserves progress so you can jump right back into your movie or show.
- 🎨 **Catppuccin Themes**: Mocha, Macchiato, Frappé, Latte, TokyoNight, Nord, Dracula, and High-Contrast modes.

---

## 🎮 Controls

| Keybinding | Action |
| :--- | :--- |
| <kbd>↑</kbd> <kbd>↓</kbd> <kbd>←</kbd> <kbd>→</kbd> / <kbd>h</kbd> <kbd>j</kbd> <kbd>k</kbd> <kbd>l</kbd> | Navigate items & grid |
| <kbd>Enter</kbd> | Select item / Launch playback |
| <kbd>Tab</kbd> | Switch Tabs (Continue Watching ↔ Favorites) |
| <kbd>Ctrl</kbd> + <kbd>P</kbd> | Switch Provider (**MovieBox** / **4KHDHub**) |
| <kbd>Ctrl</kbd> + <kbd>S</kbd> | Open Settings Hub (Player, Download Path, Theme) |
| <kbd>Ctrl</kbd> + <kbd>T</kbd> | Cycle Visual Theme |
| <kbd>d</kbd> | Download selected release / episode |
| <kbd>f</kbd> | Toggle Favorite |
| <kbd>Esc</kbd> / <kbd>q</kbd> | Back / Quit |

---

## 🔄 Updating

To check for updates and update SumanMovies, the engine, and all dependencies to the latest release:
```bash
sumanmovies --update
```
Or use the dedicated update command:
```bash
sumanmovies-update
```
Or via npm:
```bash
npm update -g sumanmovies
```

---

## 🗑️ Uninstallation

To remove all downloaded SumanMovies binaries and storage:
```bash
sumanmovies --uninstall
```

To also purge all local app data, history, and configs:
```bash
sumanmovies --uninstall --purge
```

To remove the npm package completely:
```bash
npm uninstall -g sumanmovies
```

---

## 👤 Author

**Suman**
- GitHub: [@SumanCH8514](https://github.com/SumanCH8514)
- NPM: [sumanmovies](https://www.npmjs.com/package/sumanmovies)
