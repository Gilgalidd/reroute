//! Knowing whether a newer release exists.
//!
//! Only comparison, parsing and the once-a-day rule live here; fetching is
//! in `reroute_platform::release`, and nothing is ever downloaded or
//! installed. A check tells the user a version number and offers a link,
//! no more.
//!
//! Both addresses below are constants compiled into the binary. In
//! particular the page Reroute opens is never taken from the server's
//! answer, so a tampered response can at worst show a wrong version number.

use serde::{Deserialize, Serialize};

/// Page a person opens to download a new version.
pub const RELEASES_PAGE: &str = "https://github.com/Gilgalidd/reroute/releases/latest";

/// Address that answers with the newest release as JSON.
pub const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/Gilgalidd/reroute/releases/latest";

/// What a check found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LatestRelease {
    /// Version published, without any leading `v`.
    pub version: String,
    /// Whether it is newer than the running version.
    pub newer: bool,
    /// Where to get it. Always [`RELEASES_PAGE`].
    pub page: &'static str,
}

/// Read `tag_name` out of the JSON that describes the newest release.
pub fn parse_tag(json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let tag = value.get("tag_name")?.as_str()?.trim();
    (!tag.is_empty()).then(|| tag.to_owned())
}

/// `v0.1.3` and `0.1.3` both name version `0.1.3`.
pub fn version_of(tag: &str) -> &str {
    tag.trim().trim_start_matches(['v', 'V'])
}

/// Is `latest` strictly newer than `current`? Anything that does not parse
/// as a version is treated as "not newer", so a malformed or hostile answer
/// never invites someone to "upgrade".
pub fn is_newer(latest: &str, current: &str) -> bool {
    match (
        semver::Version::parse(version_of(latest)),
        semver::Version::parse(current),
    ) {
        (Ok(latest), Ok(current)) => latest > current,
        _ => false,
    }
}

/// How long the result of an automatic check is trusted, in seconds: Reroute
/// asks at most once a day, however often it starts.
pub const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;

/// The last automatic check, kept between runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastCheck {
    /// When it was attempted, in seconds since the Unix epoch. Recorded
    /// before asking, so that a Reroute that exits before the answer comes
    /// does not ask again at the next click.
    pub checked_at: u64,
    /// The newest published version it found, if it got an answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Is an automatic check due at `now`? Yes when there was none, when the
/// last one is a day old, or when the clock has gone back since.
pub fn check_due(last: Option<&LastCheck>, now: u64) -> bool {
    last.is_none_or(|last| last.checked_at > now || now - last.checked_at >= CHECK_INTERVAL_SECS)
}

/// Where the running version stands, as the picker's update button shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum UpdateStatus {
    /// No answer yet, or checks are turned off.
    Unknown,
    /// The running version is the newest published one (or ahead of it).
    Current {
        /// Newest published version.
        version: String,
    },
    /// A newer version is published.
    Available {
        /// Newest published version.
        version: String,
    },
    /// The last check did not get an answer.
    Failed {
        /// What went wrong, for a tooltip.
        reason: String,
    },
}

impl UpdateStatus {
    /// The status when `latest` is published and `current` is running.
    pub fn compare(latest: &str, current: &str) -> Self {
        let version = version_of(latest).to_owned();
        if is_newer(&version, current) {
            Self::Available { version }
        } else {
            Self::Current { version }
        }
    }
}

/// Turn the answer of [`LATEST_RELEASE_API`] into something to display.
pub fn interpret(json: &str, current_version: &str) -> Option<LatestRelease> {
    let tag = parse_tag(json)?;
    let version = version_of(&tag).to_owned();
    Some(LatestRelease {
        newer: is_newer(&version, current_version),
        version,
        page: RELEASES_PAGE,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed down, with the fields GitHub really sends.
    const ANSWER: &str = r#"{"url":"https://api.github.com/repos/Gilgalidd/reroute/releases/1",
        "tag_name":"v0.1.3","name":"Reroute v0.1.3","draft":false,"prerelease":false,
        "published_at":"2026-09-22T16:47:38Z","body":"See CHANGELOG.md"}"#;

    #[test]
    fn reads_the_tag_and_compares_it() {
        let found = interpret(ANSWER, "0.1.2").unwrap();
        assert_eq!(found.version, "0.1.3");
        assert!(found.newer);
        assert_eq!(found.page, RELEASES_PAGE);

        let same = interpret(ANSWER, "0.1.3").unwrap();
        assert!(!same.newer, "the running version is the published one");
        let ahead = interpret(ANSWER, "0.2.0").unwrap();
        assert!(
            !ahead.newer,
            "a local build ahead of the release is not out of date"
        );
    }

    #[test]
    fn tags_may_or_may_not_start_with_v() {
        assert_eq!(version_of("v1.2.3"), "1.2.3");
        assert_eq!(version_of("V1.2.3"), "1.2.3");
        assert_eq!(version_of(" 1.2.3 "), "1.2.3");
        assert!(is_newer("v0.2.0", "0.1.9"));
        assert!(is_newer("1.0.0", "1.0.0-rc.1"));
        assert!(!is_newer("0.1.0", "0.1.0"));
    }

    #[test]
    fn an_automatic_check_is_due_once_a_day() {
        let day = CHECK_INTERVAL_SECS;
        let at = |checked_at| LastCheck {
            checked_at,
            version: None,
        };
        assert!(check_due(None, 1_000));
        assert!(!check_due(Some(&at(1_000)), 1_000 + day - 1));
        assert!(check_due(Some(&at(1_000)), 1_000 + day));
        assert!(check_due(Some(&at(5_000)), 1_000), "the clock went back");
    }

    #[test]
    fn the_status_says_current_or_available() {
        assert_eq!(
            UpdateStatus::compare("v0.2.0", "0.1.11"),
            UpdateStatus::Available {
                version: "0.2.0".into()
            }
        );
        assert_eq!(
            UpdateStatus::compare("0.1.11", "0.1.11"),
            UpdateStatus::Current {
                version: "0.1.11".into()
            }
        );
        let json = serde_json::to_string(&UpdateStatus::compare("0.2.0", "0.1.0")).unwrap();
        assert_eq!(json, r#"{"state":"available","version":"0.2.0"}"#);
    }

    #[test]
    fn nonsense_answers_never_claim_an_update() {
        assert_eq!(parse_tag("not json"), None);
        assert_eq!(parse_tag(r#"{"message":"Not Found"}"#), None);
        assert_eq!(parse_tag(r#"{"tag_name":""}"#), None);
        assert_eq!(parse_tag(r#"{"tag_name":42}"#), None);
        assert_eq!(interpret("not json", "0.1.2"), None);
        assert!(!is_newer("please-update", "0.1.2"));
        assert!(!is_newer("999.0.0", "not a version"));
        let weird = interpret(r#"{"tag_name":"nightly"}"#, "0.1.2").unwrap();
        assert!(!weird.newer);
    }
}
