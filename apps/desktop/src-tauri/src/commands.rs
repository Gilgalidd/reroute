//! IPC commands callable from the webviews.
//!
//! Every command is listed in `build.rs`, which generates an
//! `allow-<command>` permission; `capabilities/*.json` then grants each
//! window only the commands it needs.

// Tauri's command macro requires `State` and `AppHandle` by value.
#![allow(clippy::needless_pass_by_value)]

use base64::Engine;
use reroute_core::{Brand, Browser, BrowserId, Config, LaunchId, SafeUrl};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::state::AppState;
use crate::{incoming, windows};

/// A browser as the picker shows it.
#[derive(Serialize)]
pub struct BrowserView {
    id: BrowserId,
    name: String,
    launches: Vec<LaunchView>,
    /// `data:` URL of the icon, when an icon file is available.
    icon: Option<String>,
    brand: Option<Brand>,
}

#[derive(Serialize)]
struct LaunchView {
    id: LaunchId,
    name: String,
}

/// What the picker needs to render.
#[derive(Serialize)]
pub struct LaunchContext {
    url: Option<UrlView>,
    url_error: Option<String>,
    config_error: Option<String>,
    settings: reroute_core::Settings,
    browsers: Vec<BrowserView>,
    /// Where the running version stands, for the update button's colour.
    update: reroute_core::release::UpdateStatus,
}

#[derive(Serialize)]
struct UrlView {
    href: String,
    host: String,
}

fn browser_view(browser: &Browser) -> BrowserView {
    let icon =
        browser
            .icon
            .as_deref()
            .and_then(|path| match reroute_platform::icons::load_icon(path) {
                Ok(icon) => Some(format!(
                    "data:{};base64,{}",
                    icon.mime,
                    base64::engine::general_purpose::STANDARD.encode(icon.bytes)
                )),
                Err(error) => {
                    log::warn!("icon of `{}` not loaded: {error}", browser.name);
                    None
                }
            });
    BrowserView {
        id: browser.id,
        name: browser.name.clone(),
        launches: browser
            .launches
            .iter()
            .map(|l| LaunchView {
                id: l.id,
                name: l.name.clone(),
            })
            .collect(),
        icon,
        brand: Brand::from_program(&browser.path),
    }
}

/// Everything the picker window needs.
#[tauri::command]
pub fn launch_context(state: State<'_, AppState>) -> LaunchContext {
    let config = state.config();
    LaunchContext {
        url: state.pending().map(|pending| UrlView {
            href: pending.url.as_str().to_owned(),
            host: pending.url.host().to_owned(),
        }),
        url_error: state.url_error(),
        config_error: state.config_error(),
        settings: config.settings.clone(),
        browsers: config.visible_browsers().map(browser_view).collect(),
        update: state.update_status(),
    }
}

/// The picker's page has read the pending link: show its window, if a link
/// is waiting (see `windows::show_picker`).
#[tauri::command]
pub fn show_picker(app: AppHandle) -> Result<(), String> {
    windows::show_picker(&app).map_err(|e| e.to_string())
}

/// The user chose a browser for the link `url`, the one the picker showed.
#[tauri::command]
pub fn pick(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
    browser: BrowserId,
    launch: Option<LaunchId>,
    remember: bool,
) -> Result<(), String> {
    let pending = state.pending_for(&url)?;
    let url = &pending.url;
    // Build the remembered rule before opening anything: when the host
    // cannot become a rule (an IPv6 address, for one), the picker says so
    // and stays open, rather than dropping the user's choice in silence.
    let remembered = if remember {
        let mut config = state.config().clone();
        config
            .remember_domain(url.host(), browser, launch)
            .map_err(|e| format!("cannot remember {}: {e}", url.host()))?;
        Some(config)
    } else {
        None
    };
    let activation = pending.activation.as_deref();
    incoming::launch(&state.config(), url, browser, launch, activation)
        .map_err(|e| e.to_string())?;
    if let Some(config) = remembered {
        // The browser is open by now; a failed write is logged, not shown.
        if let Err(error) = state.save(config) {
            log::warn!("the browser opened, but the domain was not remembered: {error}");
        }
    }
    windows::finish_picker(&app);
    Ok(())
}

/// The user closed the picker without choosing.
#[tauri::command]
pub fn dismiss(app: AppHandle) {
    windows::finish_picker(&app);
}

/// Open the settings window from the picker. `async`, so that it runs off
/// the interface thread: on Windows, creating a window from a synchronous
/// command deadlocks WebView2, and the new window never responds (a known
/// issue, documented on Tauri's `WebviewWindowBuilder`).
#[tauri::command]
pub async fn open_settings(app: AppHandle) -> Result<(), String> {
    windows::open_settings(&app).map_err(|e| e.to_string())
}

/// What the settings window edits.
#[derive(Serialize)]
pub struct EditableConfig {
    config: Config,
    /// Why `config.toml` could not be read, when it could not. `config` then
    /// holds only the installed browsers, and saving keeps the old file as
    /// `config.toml.broken`.
    load_error: Option<String>,
}

/// The full configuration, for editing.
#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> EditableConfig {
    EditableConfig {
        config: state.config().clone(),
        load_error: state.config_error(),
    }
}

/// Validate and persist an edited configuration.
#[tauri::command]
pub fn save_config(
    app: AppHandle,
    state: State<'_, AppState>,
    config: Config,
) -> Result<(), String> {
    let background = config.settings.run_in_background;
    state.save(config)?;
    // Starting with the session follows the setting. Turning it on takes
    // full effect at the next start. Turning it off lets this Reroute exit
    // when its windows close (see `AppState::stays_in_background`): the
    // picker it kept loaded and hidden goes now, so that it does not count
    // as an open window.
    #[cfg(target_os = "linux")]
    if let Err(error) = reroute_platform::autostart::set(background) {
        log::warn!("start with the session not updated: {error}");
    }
    if !background
        && let Some(picker) = app.get_webview_window(windows::PICKER)
        && !picker.is_visible().unwrap_or(true)
        && let Err(error) = picker.close()
    {
        log::warn!("cannot close the hidden picker: {error}");
    }
    Ok(())
}

/// Add the installed browsers that `config` does not have yet.
///
/// `config` is the settings window's unsaved draft, and nothing is written
/// here: the merged draft goes back to the window, and the user keeps or
/// discards it with the other edits. Working on the saved file instead
/// would drop unsaved edits, or be overwritten by the next Save.
#[tauri::command]
pub fn discover_browsers(mut config: Config) -> Config {
    let added = config.merge_discovered(reroute_platform::discover::installed_browsers());
    log::info!("discovery added {added} browser(s)");
    config
}

/// Outcome of a dry run of the rules against a URL typed in the settings.
#[derive(Serialize)]
pub struct TestResult {
    normalized: Option<String>,
    error: Option<String>,
    matched: Option<TestMatch>,
}

#[derive(Serialize)]
struct TestMatch {
    ruleset: String,
    pattern: String,
    browser: String,
    launch: Option<String>,
}

/// Which rule (if any) would catch `url`.
#[tauri::command]
pub fn test_url(state: State<'_, AppState>, url: String) -> TestResult {
    let parsed = match SafeUrl::parse(&url) {
        Ok(parsed) => parsed,
        Err(error) => {
            return TestResult {
                normalized: None,
                error: Some(error.to_string()),
                matched: None,
            };
        }
    };
    let config = state.config();
    let matched = reroute_core::rules::find_match(&config.rulesets, &parsed).map(|m| {
        let (browser, launch) = config.resolve(m.browser, m.launch).map_or_else(
            |_| ("(unknown)".to_owned(), None),
            |(b, l)| (b.name.clone(), l.map(|l| l.name.clone())),
        );
        TestMatch {
            ruleset: config.rulesets[m.ruleset].name.clone(),
            pattern: m.pattern,
            browser,
            launch,
        }
    });
    TestResult {
        normalized: Some(parsed.as_str().to_owned()),
        error: None,
        matched,
    }
}

/// Is Reroute the system's default browser?
#[tauri::command]
pub fn default_browser_status() -> Result<bool, String> {
    reroute_platform::register::is_default().map_err(|e| e.to_string())
}

/// Ask the system to make Reroute the default browser.
#[tauri::command]
pub fn register_default_browser() -> Result<reroute_platform::register::Outcome, String> {
    if let Err(error) = reroute_platform::register::install_icons(crate::APP_ICONS) {
        log::warn!("icons not installed: {error}");
    }
    reroute_platform::register::register().map_err(|e| e.to_string())
}

/// Result of importing a Hurl settings file.
#[derive(Serialize)]
pub struct ImportReport {
    /// The draft with the import merged in, not saved yet.
    config: Config,
    browsers_added: usize,
    rulesets_added: usize,
    notes: Vec<String>,
}

/// Merge a Hurl `UserSettings.json` (pasted as text) into the settings
/// window's draft. Like [`discover_browsers`], nothing is saved here.
#[tauri::command]
pub fn import_hurl(mut config: Config, json: String) -> Result<ImportReport, String> {
    let imported = reroute_core::import::hurl::import(&json)?;
    let report = config.merge(imported.config);
    let mut notes = imported.notes;
    notes.extend(report.notes);
    Ok(ImportReport {
        config,
        browsers_added: report.browsers_added,
        rulesets_added: report.rulesets_added,
        notes,
    })
}

/// Ask which version is the newest, when the user presses an update button.
/// With the automatic check in `updates`, the only request that reaches the
/// network. Nothing is downloaded: the answer is a version number and a link.
#[tauri::command]
pub async fn check_latest_release(
    app: AppHandle,
) -> Result<reroute_core::release::LatestRelease, String> {
    tauri::async_runtime::spawn_blocking(move || crate::updates::check_now(&app))
        .await
        .map_err(|e| format!("the check could not run: {e}"))?
}

/// Open the download page in a browser: the one a rule picks for it, else
/// the first in the list. The address is a constant, never something a
/// server sent us. No click of the desktop's asked for it, so there is no
/// activation token to give the browser.
#[tauri::command]
pub fn open_release_page(state: State<'_, AppState>) -> Result<(), String> {
    let url = SafeUrl::parse(reroute_core::release::RELEASES_PAGE).map_err(|e| e.to_string())?;
    let config = state.config();
    let (browser, launch) = match incoming::rule_for(&config, &url) {
        Some(found) => (found.browser, found.launch),
        None => (
            config
                .visible_browsers()
                .next()
                .ok_or("no browser is configured")?
                .id,
            None,
        ),
    };
    incoming::launch(&config, &url, browser, launch, None).map_err(|e| e.to_string())
}

/// Static facts for the About tab.
#[derive(Serialize, Deserialize)]
pub struct AppInfo {
    version: String,
    config_path: String,
    platform: String,
}

/// Version, configuration path and OS name.
#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        config_path: state.config_path().display().to_string(),
        platform: std::env::consts::OS.to_owned(),
    }
}
