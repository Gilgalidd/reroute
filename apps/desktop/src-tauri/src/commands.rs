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
use tauri::{AppHandle, State};

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
        url: state.pending().map(|u| UrlView {
            href: u.as_str().to_owned(),
            host: u.host().to_owned(),
        }),
        url_error: state.url_error(),
        config_error: state.config_error(),
        settings: config.settings.clone(),
        browsers: config.visible_browsers().map(browser_view).collect(),
    }
}

/// The user chose a browser for the pending URL.
#[tauri::command]
pub fn pick(
    app: AppHandle,
    state: State<'_, AppState>,
    browser: BrowserId,
    launch: Option<LaunchId>,
    remember: bool,
) -> Result<(), String> {
    let url = state.pending().ok_or("there is no URL to open")?;
    let launched = {
        let config = state.config();
        incoming::launch(&config, &url, browser, launch).map_err(|e| e.to_string())
    };
    launched?;
    if remember {
        let mut config = state.config().clone();
        if let Err(error) = config
            .remember_domain(url.host(), browser, launch)
            .and_then(|()| state.save(config).map_err(reroute_core::ConfigError::Parse))
        {
            log::warn!("could not remember the domain: {error}");
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

/// Open the settings window from the picker.
#[tauri::command]
pub fn open_settings(app: AppHandle) -> Result<(), String> {
    windows::open_settings(&app).map_err(|e| e.to_string())
}

/// The full configuration, for editing.
#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Config {
    state.config().clone()
}

/// Validate and persist an edited configuration.
#[tauri::command]
pub fn save_config(state: State<'_, AppState>, config: Config) -> Result<(), String> {
    state.save(config)
}

/// Add newly installed browsers to the configuration and return it.
#[tauri::command]
pub fn discover_browsers(state: State<'_, AppState>) -> Result<Config, String> {
    let mut config = state.config().clone();
    let added = config.merge_discovered(reroute_platform::discover::installed_browsers());
    log::info!("discovery added {added} browser(s)");
    state.save(config.clone())?;
    Ok(config)
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
            }
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
    browsers_added: usize,
    rulesets_added: usize,
    notes: Vec<String>,
}

/// Merge a Hurl `UserSettings.json` (pasted as text) into the configuration.
#[tauri::command]
pub fn import_hurl(state: State<'_, AppState>, json: String) -> Result<ImportReport, String> {
    let imported = reroute_core::import::hurl::import(&json)?;
    let mut config = state.config().clone();
    let report = config.merge(imported.config);
    state.save(config)?;
    let mut notes = imported.notes;
    notes.extend(report.notes);
    Ok(ImportReport {
        browsers_added: report.browsers_added,
        rulesets_added: report.rulesets_added,
        notes,
    })
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
