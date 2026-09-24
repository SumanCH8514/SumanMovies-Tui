use sumanmovies_tui::tui::app::App;

#[cfg(not(target_os = "android"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

struct TerminalGuard;

fn restore_terminal() {
    let _ = crossterm::execute!(
        std::io::stdout(),
        crossterm::cursor::SetCursorStyle::DefaultUserShape,
        crossterm::cursor::Show,
        crossterm::event::DisableMouseCapture,
        crossterm::event::DisableFocusChange,
        crossterm::event::PopKeyboardEnhancementFlags,
        crossterm::terminal::LeaveAlternateScreen
    );
    let _ = crossterm::terminal::disable_raw_mode();
}

fn purge_stale_subtitles() {
    tokio::task::spawn_blocking(|| {
        let max_age = 24 * 60 * 60;
        let mut dirs = vec![
            sumanmovies_tui::service::resolve_subtitle_dir(),
            std::env::temp_dir().join("sumanmovies-tui/subs"),
        ];
        if sumanmovies_tui::updater::artifact::is_termux_environment()
            && let Some(home) = dirs::home_dir()
        {
            let android_storage = home.join("storage/downloads/sumanmovies_subs");
            if home.join("storage/downloads").exists() {
                dirs.push(android_storage);
            }
        }

        for dir in dirs {
            if dir.exists()
                && let Ok(entries) = std::fs::read_dir(&dir)
            {
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata()
                        && let Ok(modified) = metadata.modified()
                        && let Ok(elapsed) = modified.elapsed()
                        && elapsed.as_secs() > max_age
                    {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
        }
    });
}

fn purge_stale_update_artifacts() {
    tokio::task::spawn_blocking(|| {
        if let Ok(current_exe) = std::env::current_exe() {
            sumanmovies_tui::updater::apply::cleanup_stale_update_artifacts(&current_exe);
        }
    });
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--proxy-for-vlc") {
        let target_url = args.get(pos + 1).cloned().unwrap_or_default();
        let headers_json = args
            .get(pos + 2)
            .cloned()
            .unwrap_or_else(|| "[]".to_string());
        let sub_url = args.get(pos + 3).cloned().filter(|s| !s.is_empty());
        let headers: Vec<(String, String)> =
            serde_json::from_str(&headers_json).unwrap_or_default();
        sumanmovies_tui::proxy::run_sidecar(target_url, headers, sub_url).await;
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("sumanmovies-tui {}", env!("CARGO_PKG_VERSION"));
        println!("A terminal client for finding and streaming movies, TV shows, and anime.\n");
        println!("USAGE:");
        println!("    sumanmovies [OPTIONS]\n");
        println!("OPTIONS:");
        println!("    -h, --help           Print help information");
        println!("    -v, -V, --version    Print version information\n");
        println!("ENVIRONMENT VARIABLES:");
        println!("    SUMANMOVIES_LOG            Log level (off, error, warn, info, debug, trace)");
        println!("    SUMANMOVIES_THEME          Theme name (e.g. catppuccin, dracula, nord, etc.)");
        println!("    SUMANMOVIES_PLAYER         Preferred player (mpv, iina, vlc, android)");
        println!("    SUMANMOVIES_MPV_PATH       Custom mpv binary path");
        println!("    SUMANMOVIES_VLC_PATH       Custom vlc binary path");
        println!("    SUMANMOVIES_IINA_PATH      Custom iina-cli binary path");
        println!("    SUMANMOVIES_FOURKHDHUB_URL Custom 4KHDHub base URL");
        println!("    SUMANMOVIES_NO_IMAGE       Disable poster image queries (1/true)");
        println!(
            "    SUMANMOVIES_IMAGE_PROTOCOL Force graphics protocol (kitty, sixel, iterm2, none)"
        );
        println!("    SUMANMOVIES_CELL_SIZE      Override terminal cell size as WxH (e.g. 10x20)");
        return Ok(());
    }
    if args
        .iter()
        .any(|arg| arg == "--version" || arg == "-v" || arg == "-V")
    {
        println!("sumanmovies-tui {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    sumanmovies_tui::logging::init();

    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::capture();
        log::error!("panic: {info}\nbacktrace:\n{backtrace}");
        sumanmovies_tui::logging::flush();
        restore_terminal();
        eprintln!("{info}\n{backtrace}");
    }));

    let stdout = std::io::stdout();
    let backend =
        ratatui_crossterm::CrosstermBackend::new(std::io::BufWriter::with_capacity(65536, stdout));
    let mut terminal = ratatui::Terminal::new(backend)?;
    crossterm::terminal::enable_raw_mode()?;
    let _guard = TerminalGuard;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture,
        crossterm::event::EnableFocusChange
    )?;
    if !sumanmovies_tui::updater::artifact::is_termux_environment() {
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::event::PushKeyboardEnhancementFlags(
                crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | crossterm::event::KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            )
        );
    }

    sumanmovies_tui::cache::clean_old_cache_background();
    purge_stale_subtitles();
    purge_stale_update_artifacts();

    let mut app = App::new();
    if let Err(err) = app.run(&mut terminal).await {
        log::error!("application error: {err}");
    }
    Ok(())
}
