//! Import a `UserSettings.json` written by [Hurl](https://github.com/U-C-S/Hurl).
//!
//! Only the browser list and rulesets are converted; Hurl's Windows-specific
//! settings (background material, window size…) have no equivalent. Rule
//! prefixes map as follows: `s$` → `exact:`, `d$` → `domain:`, `r$` → `regex:`.
//! Hurl regexes use the .NET flavour; most simple ones work unchanged, and
//! any that do not compile are reported rather than silently dropped.

use std::path::PathBuf;

use serde::Deserialize;
use uuid::Uuid;

use crate::browser::{Browser, BrowserId, Launch, LaunchId};
use crate::config::Config;
use crate::rules::{Pattern, Ruleset};

/// Result of an import: the converted configuration plus human-readable
/// notes about anything that was skipped or adapted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Imported {
    /// Converted browsers and rulesets (settings are defaults).
    pub config: Config,
    /// One line per skipped or adapted item.
    pub notes: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct HurlFile {
    #[serde(default)]
    browsers: Vec<HurlBrowser>,
    #[serde(default)]
    rulesets: Vec<HurlRuleset>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct HurlBrowser {
    id: Option<Uuid>,
    name: String,
    exe_path: String,
    #[serde(default)]
    launch_args: Option<String>,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    custom_icon_path: Option<String>,
    #[serde(default)]
    is_uwp: bool,
    #[serde(default)]
    alternate_launches: Vec<HurlLaunch>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct HurlLaunch {
    id: Option<Uuid>,
    item_name: String,
    #[serde(default)]
    launch_args: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct HurlRuleset {
    #[serde(default)]
    ruleset_name: Option<String>,
    browser_id: Uuid,
    #[serde(default)]
    alternate_launch_id: Option<Uuid>,
    #[serde(default)]
    rules: Rules,
}

/// Hurl accepts either a string or an array of strings.
#[derive(Deserialize, Default)]
#[serde(untagged)]
enum Rules {
    #[default]
    None,
    One(String),
    Many(Vec<String>),
}

impl Rules {
    fn into_vec(self) -> Vec<String> {
        match self {
            Self::None => Vec::new(),
            Self::One(s) => vec![s],
            Self::Many(v) => v,
        }
    }
}

/// Convert the JSON text of a Hurl `UserSettings.json`.
pub fn import(json: &str) -> Result<Imported, String> {
    let file: HurlFile =
        serde_json::from_str(json).map_err(|e| format!("not a Hurl settings file: {e}"))?;
    let mut notes = Vec::new();
    let mut config = Config::default();

    for b in file.browsers {
        if b.is_uwp {
            notes.push(format!("skipped UWP browser `{}` (not supported)", b.name));
            continue;
        }
        let path = PathBuf::from(&b.exe_path);
        if !path.is_absolute() {
            notes.push(format!(
                "skipped browser `{}`: path `{}` is not absolute",
                b.name, b.exe_path
            ));
            continue;
        }
        let icon = b.custom_icon_path.filter(|p| {
            let local = PathBuf::from(p).is_absolute();
            if !local {
                notes.push(format!(
                    "dropped remote icon `{p}` of `{}` (network icons are not supported)",
                    b.name
                ));
            }
            local
        });
        let launches = b
            .alternate_launches
            .into_iter()
            .map(|l| Launch {
                id: LaunchId(l.id.unwrap_or_else(Uuid::new_v4)),
                name: l.item_name,
                args: split_args(l.launch_args.as_deref().unwrap_or_default()),
            })
            .collect();
        config.browsers.push(Browser {
            id: BrowserId(b.id.unwrap_or_else(Uuid::new_v4)),
            name: b.name,
            path,
            args: split_args(b.launch_args.as_deref().unwrap_or_default()),
            hidden: b.hidden,
            icon: icon.map(PathBuf::from),
            launches,
        });
    }

    for (index, set) in file.rulesets.into_iter().enumerate() {
        let name = set
            .ruleset_name
            .unwrap_or_else(|| format!("Ruleset {}", index + 1));
        let browser = BrowserId(set.browser_id);
        if config.browser(browser).is_none() {
            notes.push(format!(
                "skipped ruleset `{name}`: browser {browser} was not imported"
            ));
            continue;
        }
        let launch = set.alternate_launch_id.map(LaunchId);
        let mut patterns = Vec::new();
        for rule in set.rules.into_vec() {
            match convert_rule(&rule).parse::<Pattern>() {
                Ok(p) => patterns.push(p),
                Err(e) => notes.push(format!("skipped rule `{rule}` in `{name}`: {e}")),
            }
        }
        config.rulesets.push(Ruleset {
            name,
            browser,
            launch,
            patterns,
        });
    }

    config.validate().map_err(|e| e.to_string())?;
    Ok(Imported { config, notes })
}

/// `s$x` → `exact:x`, `d$x` → `domain:x`, `r$x` → `regex:x`, anything else
/// unchanged (Hurl treats it as exact, and so do we).
fn convert_rule(rule: &str) -> String {
    match rule.split_at_checked(2) {
        Some(("s$", rest)) => format!("exact:{rest}"),
        Some(("d$", rest)) => format!("domain:{rest}"),
        Some(("r$", rest)) => format!("regex:{rest}"),
        _ => rule.to_owned(),
    }
}

/// Hurl stores launch arguments as one command-line string; split it the
/// way a shell would, so `--profile-directory="Default"` stays one argument.
fn split_args(text: &str) -> Vec<String> {
    shlex::split(text).unwrap_or_else(|| text.split_whitespace().map(str::to_owned).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sample uses an absolute path for the host OS so the test is portable.
    const CHROME_PATH: &str = if cfg!(windows) {
        r"C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe"
    } else {
        "/opt/google/chrome/chrome"
    };

    fn sample() -> String {
        SAMPLE.replace("CHROME_PATH", CHROME_PATH)
    }

    const SAMPLE: &str = r#"{
      "LastUpdated": "22-Feb-22 2:22:22 AM",
      "Version": "0.6.2",
      "AppSettings": { "LaunchUnderMouse": false, "RuleMatching": true },
      "Browsers": [
        {
          "Id": "11111111-1111-1111-1111-111111111111",
          "Name": "Chrome",
          "ExePath": "CHROME_PATH",
          "LaunchArgs": "--profile-directory=\"Default\" %URL%",
          "CustomIconPath": "https://example.com/chrome.png",
          "AlternateLaunches": [
            { "Id": "22222222-2222-2222-2222-222222222222", "ItemName": "Incognito", "LaunchArgs": "-incognito" }
          ]
        },
        { "Name": "Firefox", "ExePath": "FirefoxUwp", "IsUwp": true },
        { "Name": "Broken", "ExePath": "relative.exe" }
      ],
      "Rulesets": [
        { "RulesetName": "Work", "BrowserId": "11111111-1111-1111-1111-111111111111",
          "AlternateLaunchId": "22222222-2222-2222-2222-222222222222",
          "Rules": ["d$*.github.com", "s$https://example.com/", "r$.*spotify.*", "https://plain.example/", "r$("] },
        { "BrowserId": "33333333-3333-3333-3333-333333333333", "Rules": "d$x.org" }
      ]
    }"#;

    #[test]
    fn converts_browsers_launches_and_rules() {
        let imported = import(&sample()).unwrap();
        let c = &imported.config;
        assert_eq!(c.browsers.len(), 1);
        let chrome = &c.browsers[0];
        assert_eq!(
            chrome.id.0.to_string(),
            "11111111-1111-1111-1111-111111111111"
        );
        assert_eq!(chrome.args, vec!["--profile-directory=Default", "%URL%"]);
        assert_eq!(chrome.launches[0].args, vec!["-incognito"]);
        assert!(chrome.icon.is_none());

        assert_eq!(c.rulesets.len(), 1);
        let set = &c.rulesets[0];
        assert_eq!(set.name, "Work");
        assert_eq!(
            set.launch.unwrap().0.to_string(),
            "22222222-2222-2222-2222-222222222222"
        );
        let texts: Vec<String> = set.patterns.iter().map(ToString::to_string).collect();
        assert_eq!(
            texts,
            vec![
                "domain:*.github.com",
                "exact:https://example.com/",
                "regex:.*spotify.*",
                "exact:https://plain.example/"
            ]
        );
    }

    #[test]
    fn reports_every_skip() {
        let imported = import(&sample()).unwrap();
        let notes = imported.notes.join("\n");
        assert!(notes.contains("UWP browser `Firefox`"), "{notes}");
        assert!(notes.contains("`Broken`"), "{notes}");
        assert!(notes.contains("remote icon"), "{notes}");
        assert!(notes.contains("skipped rule `r$(`"), "{notes}");
        assert!(
            notes.contains("33333333-3333-3333-3333-333333333333"),
            "{notes}"
        );
    }

    #[test]
    fn rejects_non_hurl_json() {
        assert!(import("{ \"Browsers\": 5 }").is_err());
        assert!(import("nope").is_err());
        assert!(import("{}").unwrap().config.browsers.is_empty());
    }

    #[test]
    fn split_args_handles_quotes() {
        assert_eq!(split_args(r#"--a="x y" -b"#), vec!["--a=x y", "-b"]);
        assert_eq!(split_args(""), Vec::<String>::new());
    }
}
