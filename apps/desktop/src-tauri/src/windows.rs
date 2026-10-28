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
    window.set_focus()?;
    #[cfg(windows)]
    watch_focus(app);
    Ok(())
}

/// How often, on Windows, the shown picker checks whether the user left it.
#[cfg(windows)]
const FOCUS_POLL: Duration = Duration::from_millis(100);

/// Windows: tell when the user leaves the picker by the window in the
/// foreground, since the picker's focus events cannot: WebView2 moves the
/// focus into the page whenever a move of the window begins, which reads as
/// a loss of focus, and dragging the picker by its title bar closed it (see
/// `reroute_platform::foreground`). One watcher at a time; it stops when the
/// picker is hidden.
#[cfg(windows)]
fn watch_focus(app: &AppHandle) {
    use std::sync::atomic::{AtomicU64, Ordering};
    /// The latest watcher; an older one stops.
    static LATEST: AtomicU64 = AtomicU64::new(0);
    let me = LATEST.fetch_add(1, Ordering::Relaxed) + 1;
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("reroute-focus".into())
        .spawn(move || {
            let mut seen_in_front = false;
            loop {
                std::thread::sleep(FOCUS_POLL);
                let visible = app
                    .get_webview_window(PICKER)
                    .and_then(|window| window.is_visible().ok())
                    .unwrap_or(false);
                if LATEST.load(Ordering::Relaxed) != me || !visible {
                    return;
                }
                let ours_in_front = reroute_platform::foreground::is_this_process();
                match after_a_look(seen_in_front, ours_in_front) {
                    Look::Waiting => {}
                    Look::InFront => seen_in_front = true,
                    Look::Left => {
                        let handle = app.clone();
                        let dismissed = app.run_on_main_thread(move || picker_lost_focus(&handle));
                        if let Err(error) = dismissed {
                            log::error!("cannot dismiss the picker: {error}");
                        }
                        return;
                    }
                }
            }
        });
    if let Err(error) = spawned {
        log::warn!("cannot watch the picker's focus: {error}");
    }
}

/// What one look at the foreground window says about the shown picker.
#[cfg(any(windows, test))]
#[derive(Debug, PartialEq, Eq)]
enum Look {
    /// One of Reroute's windows is in front: the picker, even while it is
    /// dragged, or the settings.
    InFront,
    /// Another program is in front after the picker had been: the user left.
    Left,
    /// Nothing to act on yet.
    Waiting,
}

/// `ours_in_front` says whether one of Reroute's windows is in the
/// foreground, `None` when no window is. A picker that never reached the
/// front (Windows may refuse it the focus) is not closed for being behind.
#[cfg(any(windows, test))]
fn after_a_look(seen_in_front: bool, ours_in_front: Option<bool>) -> Look {
    match ours_in_front {
        Some(true) => Look::InFront,
        Some(false) if seen_in_front => Look::Left,
        _ => Look::Waiting,
    }
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
    #[cfg(target_os = "macos")]
    update_dock(app, true);
    Ok(())
}

/// macOS: a Reroute running in the background stays out of the Dock and the
/// app switcher, as it has no taskbar button on Linux and Windows, except
/// while its settings window is open.
#[cfg(target_os = "macos")]
pub fn update_dock(app: &AppHandle, settings_open: bool) {
    let in_dock = settings_open || !app.state::<AppState>().stays_in_background();
    let policy = if in_dock {
        tauri::ActivationPolicy::Regular
    } else {
        tauri::ActivationPolicy::Accessory
    };
    if let Err(error) = app.set_activation_policy(policy) {
        log::warn!("cannot show or hide the Dock icon: {error}");
    }
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
            // Not "no window yet": running in the background, Reroute keeps
            // a hidden picker from the start.
            if !app.state::<AppState>().link_arrived()
                && let Err(error) = open_settings(&app)
            {
                log::error!("cannot open settings: {error}");
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

/// Focus moved away from the picker: its window lost the focus, or on
/// Windows another program came to the front (see `watch_focus`). Dismiss it
/// if the user asked for that behaviour, unless the focus went to our own
/// settings window.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_picker_closes_only_once_another_program_comes_to_the_front() {
        // Reroute's own window in front, the picker while it is dragged
        // among them: it stays.
        assert_eq!(after_a_look(false, Some(true)), Look::InFront);
        assert_eq!(after_a_look(true, Some(true)), Look::InFront);
        // Another program in front: the user left.
        assert_eq!(after_a_look(true, Some(false)), Look::Left);
        // Unless the picker never reached the front.
        assert_eq!(after_a_look(false, Some(false)), Look::Waiting);
        // No window in front for an instant, while Windows switches.
        assert_eq!(after_a_look(true, None), Look::Waiting);
    }
}
