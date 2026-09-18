//! Deciding what to do with an incoming URL.

use reroute_core::{BrowserId, Config, LaunchError, LaunchId, SafeUrl};

use crate::state::AppState;

/// Result of [`decide`].
#[derive(Debug)]
pub enum Decision {
    /// A rule matched and the browser was started; nothing left to do.
    Launched,
    /// No rule matched: the user must choose.
    Ask(SafeUrl),
    /// The URL failed validation; the message explains why.
    Refused(String),
}

/// Validate `raw`, then let the rules decide or defer to the picker.
pub fn decide(state: &AppState, raw: &str) -> Decision {
    let url = match SafeUrl::parse(raw) {
        Ok(url) => url,
        Err(error) => {
            log::warn!("refused incoming URL: {error}");
            return Decision::Refused(error.to_string());
        }
    };
    let config = state.config();
    if config.settings.rules_enabled {
        if let Some(found) = reroute_core::rules::find_match(&config.rulesets, &url) {
            log::info!("rule `{}` matched", found.pattern);
            match launch(&config, &url, found.browser, found.launch) {
                Ok(()) => return Decision::Launched,
                Err(error) => log::warn!("rule matched but launch failed, asking instead: {error}"),
            }
        }
    }
    Decision::Ask(url)
}

/// Resolve the browser and launch, build the plan and spawn it.
pub fn launch(
    config: &Config,
    url: &SafeUrl,
    browser: BrowserId,
    launch: Option<LaunchId>,
) -> Result<(), LaunchError> {
    let (browser, _) = config.resolve(browser, launch)?;
    let plan = browser.plan(url, launch)?;
    reroute_platform::launch::launch(&plan)
}

/// macOS delivers URLs through an event instead of the command line.
#[cfg(target_os = "macos")]
pub fn handle_opened(app: &tauri::AppHandle, urls: &[tauri::Url]) {
    use tauri::Manager;
    let Some(first) = urls.first() else { return };
    let state = app.state::<AppState>();
    match decide(&state, first.as_str()) {
        Decision::Launched => crate::windows::finish_picker(app),
        Decision::Ask(url) => state.set_pending(url),
        Decision::Refused(message) => state.set_url_error(message),
    }
    if let Err(error) = crate::windows::open_picker(app) {
        log::error!("cannot open picker: {error}");
    }
}
