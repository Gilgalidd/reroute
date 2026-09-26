//! Reroute desktop application (Tauri shell).
//!
//! Flow for a click on a link:
//!
//! 1. The OS starts `reroute <url>` (or, on macOS, sends an open-URL event).
//! 2. [`incoming::decide`] validates the URL and evaluates the rules. If a
//!    rule matches, the browser is launched and the process exits before any
//!    window is created.
//! 3. Otherwise the picker window is shown; the choice is launched through
//!    the same code path and the process exits.
//!
//! Started without arguments, Reroute opens its settings window.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod cli;
mod commands;
mod incoming;
mod state;
mod windows;

use tauri::Manager;

use crate::state::AppState;

/// Application icons embedded for the Linux user-level registration (see
/// `reroute_platform::register::install_icons`).
pub const APP_ICONS: &[(u32, &[u8])] = &[
    (32, include_bytes!("../icons/32x32.png")),
    (64, include_bytes!("../icons/64x64.png")),
    (128, include_bytes!("../icons/128x128.png")),
    (256, include_bytes!("../icons/128x128@2x.png")),
    (512, include_bytes!("../icons/icon.png")),
];

/// Entry point called from `main`.
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // GTK takes a window's application id from the program name, and the
    // desktop finds the window's icon through the entry named after it. The
    // binary is `reroute` but the entry is `Reroute.desktop`, so name the
    // program before any window exists; otherwise KDE and GNOME show a
    // generic icon.
    #[cfg(target_os = "linux")]
    glib::set_prgname(Some(reroute_platform::register::linux::APP_ID));

    let mode = cli::parse(std::env::args().skip(1));
    if mode == cli::Mode::Version {
        println!("reroute {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if mode == cli::Mode::MakeDefault {
        make_default_from_cli();
        return;
    }

    let state = AppState::load();

    if let cli::Mode::Pick(raw) = &mode {
        match incoming::decide(&state, raw) {
            incoming::Decision::Launched => return,
            incoming::Decision::Ask(url) => state.set_pending(url),
            incoming::Decision::Refused(message) => state.set_url_error(message),
        }
    }

    let app = tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::launch_context,
            commands::pick,
            commands::dismiss,
            commands::open_settings,
            commands::get_config,
            commands::save_config,
            commands::discover_browsers,
            commands::test_url,
            commands::default_browser_status,
            commands::register_default_browser,
            commands::import_hurl,
            commands::app_info,
            commands::check_latest_release,
            commands::open_release_page,
        ])
        .setup(move |app| {
            let handle = app.handle();
            match mode {
                cli::Mode::Pick(_) => windows::open_picker(handle)?,
                cli::Mode::Settings => windows::open_settings_or_wait(handle),
                cli::Mode::Version | cli::Mode::MakeDefault => {}
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == windows::PICKER
                && matches!(event, tauri::WindowEvent::Focused(false))
            {
                windows::picker_lost_focus(window.app_handle());
            }
        })
        .build(tauri::generate_context!());

    let app = match app {
        Ok(app) => app,
        Err(error) => {
            log::error!("cannot start the application: {error}");
            std::process::exit(1);
        }
    };

    app.run(|app, event| handle_run_event(app, &event));
}

/// `reroute --make-default`: for installers and scripts. Prints the outcome
/// and exits non-zero on failure.
fn make_default_from_cli() {
    if let Err(error) = reroute_platform::register::install_icons(APP_ICONS) {
        eprintln!("icons not installed: {error}");
    }
    match reroute_platform::register::register() {
        Ok(reroute_platform::register::Outcome::Done) => {
            let verified = reroute_platform::register::is_default().unwrap_or(false);
            println!("Reroute is now the default browser (verified: {verified}).");
        }
        Ok(reroute_platform::register::Outcome::NeedsUserAction(message)) => println!("{message}"),
        Err(error) => {
            eprintln!("could not register: {error}");
            std::process::exit(1);
        }
    }
}

/// macOS delivers link clicks and dock re-activation as run-loop events.
/// Other platforms have nothing to do here.
fn handle_run_event(app: &tauri::AppHandle, event: &tauri::RunEvent) {
    #[cfg(target_os = "macos")]
    match event {
        tauri::RunEvent::Opened { urls } => incoming::handle_opened(app, urls),
        tauri::RunEvent::Reopen {
            has_visible_windows: false,
            ..
        } => {
            if let Err(error) = windows::open_settings(app) {
                log::error!("cannot open settings: {error}");
            }
        }
        _ => {}
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, event);
}
