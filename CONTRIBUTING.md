# Internal Development Guide

> [!NOTE]
> **Private Project**: SumanMovies is a private personal project maintained solely by the author. External contributions, pull requests, and collaboration are not accepted.

This document serves as an internal reference for local development setup, architectural layout, and code quality workflows.

---

## 🛠️ Local Development Setup

### Prerequisites
- **Rust**: **1.90 or newer** (Edition 2024). Install or update via [rustup.rs](https://rustup.rs/).
- **Media Player**: `mpv` installed and available in PATH (or configured in Settings).

### Setup Commands
```bash
# Enable pre-commit hooks to ensure formatting and lints pass automatically
git config core.hooksPath .githooks

# Build and run locally
cargo run --release
```

---

## 📂 Project Architecture

The application is message-driven and organized into focused modules:

- **`src/main.rs`**: Application entry point, terminal initialization, cleanup guards, allocator setup.
- **`src/service.rs`**: Unified headless multi-provider client and backend orchestrator.
- **`src/tui/`**: UI state, event loop plumbing, slash commands (`commands.rs`), screens, and theme engine.
  - **`src/tui/app/`**: Application state machine (`App`). `run.rs` holds the `handle_action` dispatcher routing actions to modules (`playback.rs`, `download.rs`, `search.rs`, `requests.rs`, `navigation.rs`, `tv.rs`, `addons.rs`, etc.).
  - **`src/tui/screens/`**: Home catalog view, Details view, and Help view.
  - **`src/tui/widgets/`**: Poster rendering, settings menu, modals, badges, scrollbars.
- **`src/providers/`**: Streaming provider backends:
  - `moviebox/`: MovieBox API scraper, DASH/HLS decryption, crypto headers.
  - `fourkhdhub/`: 4KHDHub catalog scraper and HubCloud stream resolver.
  - `bdix/`: BDIX FTP mirrors (CircleFTP, DhakaFlix).
  - `tv/`: M3U/IPTV live television parser.
  - `addons/`: Community and custom provider adapter engine.
- **`src/player.rs`**: Process lifecycle and MPV IPC socket controller.
- **`src/download.rs`**: Background multi-threaded HTTP range download manager.
- **`src/cache.rs`**: Multi-tiered disk and memory caching layer.
- **`src/history.rs` & `src/favorites.rs`**: Watch state, resume timestamps, and bookmarks persistence.
- **`src/updater/`**: In-place self-updater checking GitHub Releases.

---

## 🧪 Quality Standards & Pre-Commit Hooks

Pre-commit hooks are configured in `.githooks/` to ensure consistent code quality before any commit:

- `cargo fmt --check` (Code formatting)
- `cargo clippy --all-targets --locked -- -D warnings` (Strict linting)

### Manual Verification
```bash
# Auto-format codebase
cargo fmt

# Run clippy linter
cargo clippy --all-targets -- -D warnings

# Run test suite
cargo test
```

---

## 📝 Commit Conventions

Follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` New feature or capability
- `fix:` Bug fix
- `refactor:` Code refactoring without behavioral changes
- `perf:` Performance improvements
- `style:` Formatting or aesthetic adjustments
- `docs:` Documentation updates
- `chore:` Maintenance or dependency updates
