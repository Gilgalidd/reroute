//! Reroute desktop application (Tauri shell).
//!
//! Flow for a click on a link:
//!
//! 1. The OS starts `reroute <url>` (or, on macOS, sends an open-URL event).
//! 2. On Linux, if a Reroute already runs in the background, the new process
//!    hands it the link and exits ([`resident`]); that Reroute continues
//!    from step 3 with its picker already loaded.
//! 3. [`incoming::route`] validates the URL and evaluates the rules. If a
//!    rule matches, the browser is launched and no window is shown.
//! 4. Otherwise the picker is shown; the choice is launched through the same
//!    code path. Then the process exits, or, running in the background,
//!    hides the picker for the next click.
//!
//! Started without arguments, Reroute opens its settings window.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod cli;
mod commands;
mod incoming;
#[cfg(target_os = "linux")]
mod resident;
mod state;
mod updates;
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
    #[cfg(target_os = "linux")]
    use_shared_memory_rendering();

    let mode = cli::parse(std::env::args().skip(1));
    if mode == cli::Mode::Version {
        println!("reroute {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if mode == cli::Mode::MakeDefault {
        make_default_from_cli();
        return;
    }

    // The desktop's activation token for this click, which the browser
    // opening the link needs to come to the front.
    let activation = std::env::var(reroute_platform::launch::ACTIVATION_TOKEN)
        .ok()
        .filter(|token| reroute_core::control::is_activation_token(token));

    // A Reroute running in the background answers at once.
    #[cfg(target_os = "linux")]
    if resident::hand_over(&mode, activation.as_deref()) {
        return;
    }

    let state = AppState::load();
    updates::restore(&state);

    #[cfg(target_os = "linux")]
    let listener = match resident::claim(&mode, activation.as_deref(), &state) {
        resident::Role::Resident(listener) => {
            state.set_resident();
            Some(listener)
        }
        resident::Role::Alone => None,
        resident::Role::Done => return,
    };

    let picker_needed = match &mode {
        cli::Mode::Pick(raw) => incoming::route(&state, raw, activation),
        _ => false,
    };
    let resident = state.stays_in_background();
    if !resident && !picker_needed && mode != cli::Mode::Settings {
        // A rule opened the link, or a background start that is not wanted.
        return;
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
            commands::show_picker,
        ])
        .setup(move |app| {
            let handle = app.handle();
            if picker_needed {
                windows::request_picker(handle)?;
            } else if mode == cli::Mode::Settings {
                windows::open_settings_or_wait(handle);
            }
            #[cfg(target_os = "linux")]
            if let Some(listener) = listener {
                windows::preload_picker(handle)?;
                resident::serve(listener, handle);
                if let Err(error) = reroute_platform::autostart::set(true) {
                    log::warn!("cannot start with the session: {error}");
                }
            }
            updates::refresh_if_due(handle);
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

/// Turn WebKitGTK's DMA-BUF renderer off, unless the user set the variable.
///
/// That renderer brings the GPU up when the first webview is created: about
/// 0.9 s of the 1.9 s the picker took to appear after a click, on a KDE
/// Wayland laptop, for a small window that needs no GPU. Without it WebKit
/// draws in shared memory. `WEBKIT_DISABLE_DMABUF_RENDERER=0` in the
/// environment brings the renderer back. The browsers Reroute starts do not
/// inherit the variable: a WebKit-based one, such as GNOME Web, keeps its GPU.
#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
fn use_shared_memory_rendering() {
    const NAME: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
    if std::env::var_os(NAME).is_some() {
        return;
    }
    // SAFETY: `set_var` is unsafe because another thread could read the
    // environment at the same moment. There is none yet: this runs at the
    // start of `run`, before Tauri, GTK or WebKit start any thread, and the
    // code before it (logging, naming the program) starts none either.
    unsafe { std::env::set_var(NAME, "1") };
    reroute_platform::launch::keep_from_browsers(NAME);
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

/// Keep running when the last window closes while Reroute stays in the
/// background. macOS also delivers link clicks and dock re-activation here.
fn handle_run_event(app: &tauri::AppHandle, event: &tauri::RunEvent) {
    // `code: None` is the last window closing; an explicit `exit` has a code.
    if let tauri::RunEvent::ExitRequested {
        api, code: None, ..
    } = event
        && app.state::<AppState>().stays_in_background()
    {
        api.prevent_exit();
    }
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
}
