//! Deciding what to do with an incoming URL.

use reroute_core::{BrowserId, Config, LaunchError, LaunchId, Match, SafeUrl};

use crate::state::AppState;

/// Route the link `raw` from a click: a matching rule opens it in its
/// browser, otherwise it waits for the picker, which also shows why a link
/// was refused. Returns whether the picker is needed.
///
/// `activation` is the activation token the desktop gave the click; the
/// browser that opens the link gets it.
pub fn route(state: &AppState, raw: &str, activation: Option<String>) -> bool {
    let url = match SafeUrl::parse(raw) {
        Ok(url) => url,
        Err(error) => {
            log::warn!("refused incoming URL: {error}");
            state.set_url_error(error.to_string());
            return true;
        }
    };
    let config = state.config();
    if let Some(found) = rule_for(&config, &url) {
        log::info!("rule `{}` matched", found.pattern);
        let activation = activation.as_deref();
        match launch(&config, &url, found.browser, found.launch, activation) {
            Ok(()) => return false,
            Err(error) => log::warn!("rule matched but launch failed, asking instead: {error}"),
        }
    }
    state.set_pending(url, activation);
    true
}

/// The rule that decides for `url`: the first match, when rules are on.
pub fn rule_for(config: &Config, url: &SafeUrl) -> Option<Match> {
    if !config.settings.rules_enabled {
        return None;
    }
    reroute_core::rules::find_match(&config.rulesets, url)
}

/// Resolve the browser and launch, build the plan and spawn it, handing the
/// browser the click's `activation` token.
pub fn launch(
    config: &Config,
    url: &SafeUrl,
    browser: BrowserId,
    launch: Option<LaunchId>,
    activation: Option<&str>,
) -> Result<(), LaunchError> {
    let (browser, _) = config.resolve(browser, launch)?;
    let plan = browser.plan(url, launch)?;
    reroute_platform::launch::launch(&plan, activation)
}

/// macOS delivers URLs through an event instead of the command line.
#[cfg(target_os = "macos")]
pub fn handle_opened(app: &tauri::AppHandle, urls: &[tauri::Url]) {
    use tauri::Manager;
    let Some(first) = urls.first() else { return };
    if !route(&app.state::<AppState>(), first.as_str(), None) {
        // A rule chose the browser: no picker for this link.
        crate::windows::finish_picker(app);
        return;
    }
    if let Err(error) = crate::windows::request_picker(app) {
        log::error!("cannot open picker: {error}");
    }
}
