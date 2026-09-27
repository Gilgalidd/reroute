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

#[cfg(test)]
mod tests {
    use super::*;
    use reroute_core::ConfigStore;

    /// A state whose `config.toml` is `toml`.
    fn state_with(toml: &str) -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let store = ConfigStore::in_dir(dir.path());
        std::fs::write(store.path(), toml).unwrap();
        (dir, AppState::open(store, Vec::new))
    }

    #[test]
    fn a_refused_link_goes_to_the_picker_with_its_reason() {
        let (_dir, state) = state_with("version = 1\n");
        assert!(route(&state, "javascript:alert(1)", None));
        assert!(state.pending().is_none());
        assert!(state.url_error().unwrap().contains("javascript"));
    }

    /// One browser, `program`, and a rule that sends `rule.test` to it.
    #[cfg(unix)]
    fn with_a_rule(program: &str, rules_enabled: bool) -> (tempfile::TempDir, AppState) {
        state_with(&format!(
            r#"version = 1

[settings]
rules_enabled = {rules_enabled}

[[browsers]]
id = "6d1a4b1e-6a0e-4a5f-9c1b-2f0f5f2f1a11"
name = "Browser"
path = "{program}"

[[rulesets]]
name = "Tests"
browser = "6d1a4b1e-6a0e-4a5f-9c1b-2f0f5f2f1a11"
patterns = ["domain:rule.test"]
"#
        ))
    }

    #[cfg(unix)]
    #[test]
    fn a_rule_opens_the_link_without_the_picker() {
        let (_dir, state) = with_a_rule("/usr/bin/true", true);
        assert!(!route(
            &state,
            "https://rule.test/page",
            Some("kwin-1".into())
        ));
        assert!(state.pending().is_none());
    }

    #[cfg(unix)]
    #[test]
    fn a_link_no_rule_takes_waits_for_the_picker_with_its_token() {
        let (_dir, state) = with_a_rule("/usr/bin/true", true);
        assert!(route(&state, "https://other.test/", Some("kwin-1".into())));
        let pending = state.pending().unwrap();
        assert_eq!(pending.url.as_str(), "https://other.test/");
        assert_eq!(pending.activation.as_deref(), Some("kwin-1"));
    }

    #[cfg(unix)]
    #[test]
    fn with_rules_off_the_picker_decides() {
        let (_dir, state) = with_a_rule("/usr/bin/true", false);
        assert!(route(&state, "https://rule.test/page", None));
    }

    #[cfg(unix)]
    #[test]
    fn a_rule_whose_browser_is_gone_falls_back_to_the_picker() {
        let (_dir, state) = with_a_rule("/nonexistent/browser", true);
        assert!(route(&state, "https://rule.test/page", None));
        assert!(state.pending().is_some(), "the link is not lost");
    }
}
