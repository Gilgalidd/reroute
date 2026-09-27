//! Window management. Two windows exist: the picker (small, always on top,
//! closes on focus loss) and the settings window.
//!
//! The picker is created hidden and shown by its own page once the page has
//! read the pending link ([`show_picker`]), so it never appears empty or
//! with the previous link. A Reroute running in the background keeps it
//! loaded and hidden between clicks.

use std::time::Duration;

use tauri::{AppHandle, LogicalPosition, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::state::AppState;

/// Label of the picker window.
pub const PICKER: &str = "picker";
/// Label of the settings window.
pub const SETTINGS: &str = "settings";

const PICKER_WIDTH: f64 = 560.0;
const PICKER_HEIGHT: f64 = 340.0;

fn theme(settings: &reroute_core::Settings) -> Option<tauri::Theme> {
    match settings.theme {
        reroute_core::Theme::Auto => None,
        reroute_core::Theme::Light => Some(tauri::Theme::Light),
        reroute_core::Theme::Dark => Some(tauri::Theme::Dark),
    }
}

/// If the page has not asked to be shown by then, show the picker anyway:
/// a link must never go unanswered because a page failed to load.
const SHOW_AT_THE_LATEST: Duration = Duration::from_millis(1500);

/// Ask for the picker for the pending link. An existing picker is told to
/// read its context again; a new one reads it as it loads. Either way the
/// page then calls [`show_picker`].
pub fn request_picker(app: &AppHandle) -> tauri::Result<()> {
    app.state::<AppState>().want_picker();
    if app.get_webview_window(PICKER).is_some() {
        tauri::Emitter::emit_to(app, PICKER, "context-changed", ())?;
    } else {
        create_picker(app)?;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(SHOW_AT_THE_LATEST);
        let handle = app.clone();
        let shown = app.run_on_main_thread(move || {
            if let Err(error) = show_picker(&handle) {
                log::error!("cannot show the picker: {error}");
            }
        });
        if let Err(error) = shown {
            log::error!("cannot show the picker: {error}");
        }
    });
    Ok(())
}

/// Load the picker without showing it, so that the next click finds WebKit
/// and the page ready (Reroute running in the background).
pub fn preload_picker(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window(PICKER).is_none() {
        create_picker(app)?;
    }
    Ok(())
}

fn create_picker(app: &AppHandle) -> tauri::Result<()> {
    let settings = app.state::<AppState>().config().settings.clone();
    WebviewWindowBuilder::new(app, PICKER, WebviewUrl::App("index.html#/picker".into()))
        .title("Reroute")
        .inner_size(PICKER_WIDTH, PICKER_HEIGHT)
        .min_inner_size(420.0, 280.0)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .theme(theme(&settings))
        .build()?;
    Ok(())
}

/// The page has read the pending link: show the picker, where the settings
/// ask for it, if a link is waiting. Does nothing otherwise, so that the
/// preloaded page and the safety timer can both call it freely.
pub fn show_picker(app: &AppHandle) -> tauri::Result<()> {
    if !app.state::<AppState>().take_picker_wish() {
        return Ok(());
    }
    let Some(window) = app.get_webview_window(PICKER) else {
        return Ok(());
    };
    let under_cursor = app.state::<AppState>().config().settings.open_under_cursor;
    match under_cursor.then(|| position_under_cursor(app)).flatten() {
        Some((x, y)) => window.set_position(LogicalPosition::new(x, y))?,
        None => window.center()?,
    }
    window.show()?;
    window.set_focus()
}

/// Logical top-left position that puts the picker under the mouse pointer
/// while keeping it inside the monitor's work area.
fn position_under_cursor(app: &AppHandle) -> Option<(f64, f64)> {
    let cursor = app.cursor_position().ok()?;
    let monitor = app.monitor_from_point(cursor.x, cursor.y).ok().flatten()?;
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let (left, top) = (
        f64::from(area.position.x) / scale,
        f64::from(area.position.y) / scale,
    );
    let (right, bottom) = (
        left + f64::from(area.size.width) / scale,
        top + f64::from(area.size.height) / scale,
    );
    let x = (cursor.x / scale - PICKER_WIDTH / 2.0).clamp(left, (right - PICKER_WIDTH).max(left));
    let y = (cursor.y / scale - 40.0).clamp(top, (bottom - PICKER_HEIGHT).max(top));
    Some((x, y))
}

/// Show the settings window, creating it if needed.
pub fn open_settings(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(SETTINGS) {
        return window.set_focus();
    }
    let settings = app.state::<AppState>().config().settings.clone();
    WebviewWindowBuilder::new(
        app,
        SETTINGS,
        WebviewUrl::App("index.html#/settings".into()),
    )
    .title("Reroute settings")
    .inner_size(880.0, 640.0)
    .min_inner_size(640.0, 480.0)
    .center()
    .theme(theme(&settings))
    .build()?;
    Ok(())
}

/// Open the settings window, except on macOS where a link click starts the
/// app *without* arguments and delivers the URL a moment later as an event.
/// There we wait briefly and only open settings if no URL arrived.
pub fn open_settings_or_wait(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        let app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(400));
            if app.webview_windows().is_empty() {
                if let Err(error) = open_settings(&app) {
                    log::error!("cannot open settings: {error}");
                }
            }
        });
    }
    #[cfg(not(target_os = "macos"))]
    if let Err(error) = open_settings(app) {
        log::error!("cannot open settings: {error}");
    }
}

/// The picker is done (choice made or dismissed). A Reroute running in the
/// background hides it for the next click; otherwise it closes, and the
/// process exits unless the settings window is still in use.
pub fn finish_picker(app: &AppHandle) {
    let state = app.state::<AppState>();
    state.clear_pending();
    if state.stays_in_background() {
        if let Some(window) = app.get_webview_window(PICKER)
            && let Err(error) = window.hide()
        {
            log::warn!("cannot hide picker: {error}");
        }
        return;
    }
    if app.get_webview_window(SETTINGS).is_some() {
        if let Some(window) = app.get_webview_window(PICKER)
            && let Err(error) = window.close()
        {
            log::warn!("cannot close picker: {error}");
        }
    } else {
        app.exit(0);
    }
}

/// Focus moved away from the picker. Dismiss it if the user asked for that
/// behaviour, unless the focus went to our own settings window.
pub fn picker_lost_focus(app: &AppHandle) {
    let close = app
        .state::<AppState>()
        .config()
        .settings
        .close_on_focus_loss;
    // Hiding the picker takes its focus away too: it is already dismissed,
    // and dismissing it again could drop the link of the next click.
    let visible = app
        .get_webview_window(PICKER)
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);
    if !close || !visible {
        return;
    }
    let settings_focused = app
        .get_webview_window(SETTINGS)
        .and_then(|w| w.is_focused().ok())
        .unwrap_or(false);
    if settings_focused {
        return;
    }
    log::info!("picker lost focus; dismissing");
    finish_picker(app);
}
