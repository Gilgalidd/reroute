//! The one network request Reroute makes: asking which version is the
//! newest, when the user presses an update button, and by itself at most
//! once a day when the user leaves `check_for_updates` on.
//!
//! It is a single HTTPS GET of a fixed address, with a short timeout and a
//! small size limit. Nothing about the user is sent: no version, no
//! configuration, no identifier. Nothing is downloaded or executed as a
//! result; the answer is only compared with the running version.

use std::time::Duration;

use std::path::Path;

use reroute_core::release::{LATEST_RELEASE_API, LastCheck, LatestRelease, interpret};

use crate::PlatformError;

/// Give up rather than keep a settings window waiting.
const TIMEOUT: Duration = Duration::from_secs(10);

/// Refuse an answer larger than this; the real one is a few kilobytes.
const MAX_ANSWER: u64 = 256 * 1024;

/// A neutral user agent: enough for the server to answer, nothing about
/// this machine or this installation.
const USER_AGENT: &str = "reroute";

/// Ask which version is newest and compare it with `current_version`.
///
/// This call blocks, so callers run it off the interface thread.
pub fn latest_release(current_version: &str) -> Result<LatestRelease, PlatformError> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .user_agent(USER_AGENT)
        .https_only(true)
        .build()
        .into();

    let body = agent
        .get(LATEST_RELEASE_API)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| PlatformError::Os(describe(&e)))?
        .body_mut()
        .with_config()
        .limit(MAX_ANSWER)
        .read_to_string()
        .map_err(|e| PlatformError::Os(format!("could not read the answer: {e}")))?;

    interpret(&body, current_version)
        .ok_or_else(|| PlatformError::Os("the answer did not name a version".into()))
}

/// File in [`crate::paths::cache_dir`] that remembers the last automatic
/// check between runs.
pub const LAST_CHECK_FILE: &str = "last-version-check.json";

/// The last automatic check recorded in `dir`, if any. A missing, damaged
/// or oversized file reads as none, which only means asking again.
pub fn load_last_check(dir: &Path) -> Option<LastCheck> {
    let path = dir.join(LAST_CHECK_FILE);
    if std::fs::metadata(&path).ok()?.len() > 4096 {
        return None;
    }
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// Record `check` in `dir`.
pub fn store_last_check(dir: &Path, check: &LastCheck) -> Result<(), PlatformError> {
    let os = |e: std::io::Error| PlatformError::Os(format!("{}: {e}", dir.display()));
    std::fs::create_dir_all(dir).map_err(os)?;
    let text = serde_json::to_string(check).map_err(|e| PlatformError::Os(e.to_string()))?;
    std::fs::write(dir.join(LAST_CHECK_FILE), text).map_err(os)
}

/// Say what went wrong in terms a person can act on.
fn describe(error: &ureq::Error) -> String {
    match error {
        ureq::Error::StatusCode(404) => {
            "no published release was found (are the releases private?)".to_owned()
        }
        ureq::Error::StatusCode(code) => format!("the server answered {code}"),
        ureq::Error::Timeout(_) => "the server did not answer in time".to_owned(),
        other => format!("could not reach the server: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_are_explained_rather_than_dumped() {
        assert!(describe(&ureq::Error::StatusCode(404)).contains("private"));
        assert!(describe(&ureq::Error::StatusCode(500)).contains("500"));
    }

    #[test]
    fn the_last_check_survives_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_last_check(dir.path()), None);
        let check = LastCheck {
            checked_at: 1_790_000_000,
            version: Some("0.1.12".into()),
        };
        store_last_check(dir.path(), &check).unwrap();
        assert_eq!(load_last_check(dir.path()), Some(check));
        std::fs::write(dir.path().join(LAST_CHECK_FILE), "not json").unwrap();
        assert_eq!(load_last_check(dir.path()), None, "damaged reads as none");
    }

    #[test]
    fn the_address_is_https_and_fixed() {
        assert!(LATEST_RELEASE_API.starts_with("https://"));
        assert!(reroute_core::release::RELEASES_PAGE.starts_with("https://"));
    }
}
