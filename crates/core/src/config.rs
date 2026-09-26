//! The configuration document.
//!
//! [`Config`] is the in-memory form of `config.toml`. Parsing is strict:
//! unknown keys are errors (a typo must not silently disable a rule) and
//! [`Config::validate`] enforces referential integrity so that the rest of
//! the program can rely on every id resolving.

use serde::{Deserialize, Serialize};

use crate::browser::{Browser, BrowserId, Launch, LaunchId};
use crate::error::ConfigError;
use crate::rules::Ruleset;

/// Format version written to new files. Bump when a change is not
/// backwards-readable; the store refuses files newer than this.
pub const CONFIG_VERSION: u32 = 1;

/// Colour scheme of the UI.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Follow the operating system.
    #[default]
    Auto,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
}

/// How the picker lays out the browsers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PickerLayout {
    /// Square tiles with a large icon, as many per row as the window holds.
    #[default]
    Tiles,
    /// One browser per line, its name written in full.
    List,
    /// The same list, split in two columns.
    TwoColumns,
}

/// Behavioural switches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    /// Evaluate rulesets before showing the picker.
    pub rules_enabled: bool,
    /// Show the picker next to the mouse pointer instead of screen centre.
    pub open_under_cursor: bool,
    /// Close the picker (without opening anything) when it loses focus.
    pub close_on_focus_loss: bool,
    /// Show the "remember for this domain" checkbox in the picker.
    pub offer_remember: bool,
    /// Colour scheme.
    pub theme: Theme,
    /// Tiles or a list in the picker.
    pub picker_layout: PickerLayout,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            rules_enabled: true,
            open_under_cursor: false,
            close_on_focus_loss: true,
            offer_remember: true,
            theme: Theme::Auto,
            picker_layout: PickerLayout::Tiles,
        }
    }
}

/// Outcome of [`Config::merge`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct MergeReport {
    /// Browsers appended.
    pub browsers_added: usize,
    /// Rulesets appended.
    pub rulesets_added: usize,
    /// Human-readable remarks about redirected or skipped items.
    pub notes: Vec<String>,
}

/// Everything Reroute persists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Format version, see [`CONFIG_VERSION`].
    #[serde(default = "default_version")]
    pub version: u32,
    /// Behavioural switches.
    #[serde(default)]
    pub settings: Settings,
    /// Browser catalogue, in display order.
    #[serde(default)]
    pub browsers: Vec<Browser>,
    /// Rulesets, in priority order.
    #[serde(default)]
    pub rulesets: Vec<Ruleset>,
}

fn default_version() -> u32 {
    CONFIG_VERSION
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            settings: Settings::default(),
            browsers: Vec::new(),
            rulesets: Vec::new(),
        }
    }
}

impl Config {
    /// Parse a TOML document and validate it.
    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(text).map_err(|e| ConfigError::Parse(e.to_string()))?;
        if config.version > CONFIG_VERSION {
            return Err(ConfigError::UnsupportedVersion {
                found: config.version,
                supported: CONFIG_VERSION,
            });
        }
        config.validate()?;
        Ok(config)
    }

    /// Serialise to TOML. Only fails if a value cannot be represented, which
    /// cannot happen for a validated configuration.
    pub fn to_toml(&self) -> Result<String, ConfigError> {
        toml::to_string_pretty(self).map_err(|e| ConfigError::Parse(e.to_string()))
    }

    /// Check referential integrity: unique ids, absolute paths, and every
    /// ruleset pointing at an existing browser/launch.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let mut seen = std::collections::HashSet::new();
        for b in &self.browsers {
            if b.name.trim().is_empty() {
                return Err(ConfigError::EmptyBrowserName(b.id.to_string()));
            }
            if !b.path.is_absolute() {
                return Err(ConfigError::RelativePath {
                    name: b.name.clone(),
                    path: b.path.display().to_string(),
                });
            }
            if !seen.insert(b.id.0) {
                return Err(ConfigError::DuplicateId(b.id.to_string()));
            }
            for l in &b.launches {
                if !seen.insert(l.id.0) {
                    return Err(ConfigError::DuplicateId(l.id.to_string()));
                }
            }
        }
        for set in &self.rulesets {
            let browser = self
                .browser(set.browser)
                .ok_or_else(|| ConfigError::UnknownBrowser {
                    ruleset: set.name.clone(),
                    browser: set.browser.to_string(),
                })?;
            if let Some(launch) = set.launch {
                if browser.launch(launch).is_none() {
                    return Err(ConfigError::UnknownLaunch {
                        ruleset: set.name.clone(),
                        launch: launch.to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Look up a browser by id.
    pub fn browser(&self, id: BrowserId) -> Option<&Browser> {
        self.browsers.iter().find(|b| b.id == id)
    }

    /// Browsers shown in the picker, in order.
    pub fn visible_browsers(&self) -> impl Iterator<Item = &Browser> {
        self.browsers.iter().filter(|b| !b.hidden)
    }

    /// Resolve a browser and optional launch, as used by the picker.
    pub fn resolve(
        &self,
        browser: BrowserId,
        launch: Option<LaunchId>,
    ) -> Result<(&Browser, Option<&Launch>), crate::error::LaunchError> {
        use crate::error::LaunchError;
        let b = self
            .browser(browser)
            .ok_or_else(|| LaunchError::UnknownBrowser(browser.to_string()))?;
        let l = match launch {
            Some(id) => Some(
                b.launch(id)
                    .ok_or_else(|| LaunchError::UnknownLaunch(id.to_string()))?,
            ),
            None => None,
        };
        Ok((b, l))
    }

    /// Add discovered browsers the configuration does not have yet.
    ///
    /// An entry is recognised by its executable **and** its arguments, so
    /// the private entry of a browser counts as an entry of its own rather
    /// than a duplicate of the ordinary one. Entries already there keep
    /// their id and their name, so rules and renames survive a new
    /// detection. Returns how many entries were added.
    pub fn merge_discovered(&mut self, discovered: Vec<Browser>) -> usize {
        let mut added = 0;
        for candidate in discovered {
            let known = self
                .browsers
                .iter()
                .any(|b| b.path == candidate.path && b.args == candidate.args);
            if !known {
                self.browsers.push(candidate);
                added += 1;
            }
        }
        added
    }

    /// Merge another configuration (typically an import) into this one.
    ///
    /// Browsers are matched on their executable path: a browser that already
    /// exists keeps its id, and rulesets from `other` are rewritten to point
    /// at it. Launch references to a skipped browser are dropped with a note.
    /// Settings are not merged.
    pub fn merge(&mut self, other: Config) -> MergeReport {
        let mut report = MergeReport::default();
        let mut browser_map = std::collections::HashMap::new();
        let mut kept_launches = std::collections::HashSet::new();
        for browser in other.browsers {
            if let Some(existing) = self.browsers.iter().find(|b| b.path == browser.path) {
                browser_map.insert(browser.id, existing.id);
                report.notes.push(format!(
                    "browser `{}` already exists as `{}`; rules were redirected to it",
                    browser.name, existing.name
                ));
            } else {
                browser_map.insert(browser.id, browser.id);
                kept_launches.extend(browser.launches.iter().map(|l| l.id));
                self.browsers.push(browser);
                report.browsers_added += 1;
            }
        }
        for mut set in other.rulesets {
            let Some(&browser) = browser_map.get(&set.browser) else {
                report
                    .notes
                    .push(format!("skipped ruleset `{}`: unknown browser", set.name));
                continue;
            };
            set.browser = browser;
            if let Some(launch) = set.launch {
                if !kept_launches.contains(&launch) {
                    report.notes.push(format!(
                        "ruleset `{}` now uses the default launch (its profile belonged to a browser that already existed)",
                        set.name
                    ));
                    set.launch = None;
                }
            }
            self.rulesets.push(set);
            report.rulesets_added += 1;
        }
        report
    }

    /// Prepend a ruleset that sends `host` (and its subdomains) to the given
    /// browser. Used by the picker's "remember for this domain" box. If a
    /// ruleset already targets exactly this browser/launch, the pattern is
    /// appended to it instead.
    pub fn remember_domain(
        &mut self,
        host: &str,
        browser: BrowserId,
        launch: Option<LaunchId>,
    ) -> Result<(), ConfigError> {
        let pattern: crate::rules::Pattern =
            format!("domain:*.{host}")
                .parse()
                .map_err(|source| ConfigError::Pattern {
                    ruleset: "remembered".into(),
                    source,
                })?;
        if let Some(set) = self
            .rulesets
            .iter_mut()
            .find(|s| s.browser == browser && s.launch == launch)
        {
            if !set.patterns.contains(&pattern) {
                set.patterns.push(pattern);
            }
            return Ok(());
        }
        let name = self.browser(browser).map_or_else(
            || "Remembered".to_owned(),
            |b| format!("Remembered: {}", b.name),
        );
        self.rulesets.push(Ruleset {
            name,
            browser,
            launch,
            patterns: vec![pattern],
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Pattern;
    use crate::testutil::abs;

    fn sample() -> Config {
        let launch = Launch {
            id: LaunchId::new(),
            name: "Private".into(),
            args: vec!["-p".into()],
        };
        let firefox = Browser {
            launches: vec![launch.clone()],
            ..Browser::new("Firefox", abs("/usr/bin/firefox"))
        };
        let chrome = Browser::new("Chrome", abs("/usr/bin/chrome"));
        let rulesets = vec![Ruleset {
            name: "Work".into(),
            browser: firefox.id,
            launch: Some(launch.id),
            patterns: vec!["domain:*.github.com".parse().unwrap()],
        }];
        Config {
            browsers: vec![firefox, chrome],
            rulesets,
            ..Config::default()
        }
    }

    #[test]
    fn toml_round_trip() {
        let c = sample();
        let text = c.to_toml().unwrap();
        let back = Config::from_toml(&text).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn default_document_is_minimal_and_valid() {
        let text = Config::default().to_toml().unwrap();
        assert!(text.contains("version = 1"));
        assert_eq!(Config::from_toml(&text).unwrap(), Config::default());
        assert_eq!(Config::from_toml("").unwrap(), Config::default());
    }

    #[test]
    fn picker_layout_is_optional_and_spelled_in_kebab_case() {
        let older = Config::from_toml("[settings]\ntheme = \"dark\"\n").unwrap();
        assert_eq!(older.settings.picker_layout, PickerLayout::Tiles);
        for (text, layout) in [
            ("tiles", PickerLayout::Tiles),
            ("list", PickerLayout::List),
            ("two-columns", PickerLayout::TwoColumns),
        ] {
            let config = Config::from_toml(&format!("[settings]\npicker_layout = \"{text}\"\n"));
            assert_eq!(config.unwrap().settings.picker_layout, layout, "{text}");
        }
        assert!(Config::from_toml("[settings]\npicker_layout = \"grid\"\n").is_err());
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let err = Config::from_toml("[settings]\nrules_enabld = true\n").unwrap_err();
        assert!(matches!(err, ConfigError::Parse(_)), "{err}");
    }

    #[test]
    fn newer_version_is_refused() {
        let err = Config::from_toml("version = 99\n").unwrap_err();
        assert_eq!(
            err,
            ConfigError::UnsupportedVersion {
                found: 99,
                supported: CONFIG_VERSION
            }
        );
    }

    #[test]
    fn dangling_browser_and_launch_are_refused() {
        let mut c = sample();
        c.rulesets[0].browser = BrowserId::new();
        assert!(matches!(
            c.validate(),
            Err(ConfigError::UnknownBrowser { .. })
        ));
        let mut c = sample();
        c.rulesets[0].launch = Some(LaunchId::new());
        assert!(matches!(
            c.validate(),
            Err(ConfigError::UnknownLaunch { .. })
        ));
    }

    #[test]
    fn duplicate_ids_relative_paths_and_empty_names_are_refused() {
        let mut c = sample();
        c.browsers[1].id = c.browsers[0].id;
        assert!(matches!(c.validate(), Err(ConfigError::DuplicateId(_))));
        let mut c = sample();
        c.browsers[1].path = "chrome".into();
        assert!(matches!(
            c.validate(),
            Err(ConfigError::RelativePath { .. })
        ));
        let mut c = sample();
        c.browsers[1].name = "  ".into();
        assert!(matches!(
            c.validate(),
            Err(ConfigError::EmptyBrowserName(_))
        ));
    }

    #[test]
    fn invalid_pattern_in_file_is_a_parse_error() {
        let mut text = sample().to_toml().unwrap();
        text = text.replace("domain:*.github.com", "regex:(");
        assert!(matches!(
            Config::from_toml(&text),
            Err(ConfigError::Parse(_))
        ));
    }

    #[test]
    fn merge_discovered_skips_known_paths() {
        let mut c = sample();
        let known_id = c.browsers[0].id;
        let added = c.merge_discovered(vec![
            Browser::new("Firefox again", abs("/usr/bin/firefox")),
            Browser::new("Brave", abs("/usr/bin/brave")),
        ]);
        assert_eq!(added, 1);
        assert_eq!(c.browsers.len(), 3);
        assert_eq!(c.browsers[0].id, known_id);
        assert_eq!(c.browsers[2].name, "Brave");
    }

    #[test]
    fn a_private_entry_is_not_taken_for_its_ordinary_one() {
        let mut c = sample();
        let firefox = c.browsers[0].id;
        let renamed = "My Firefox";
        c.browsers[0].name = renamed.to_owned();

        let plain = Browser::new("Firefox", abs("/usr/bin/firefox"));
        let private = Browser {
            name: "Firefox (Private)".into(),
            args: vec!["--private-window".into()],
            ..Browser::new("Firefox", abs("/usr/bin/firefox"))
        };
        assert_eq!(
            c.merge_discovered(vec![plain, private]),
            1,
            "only the private entry is new"
        );

        assert_eq!(c.browsers[0].id, firefox, "the known entry keeps its id");
        assert_eq!(c.browsers[0].name, renamed, "and the name it was given");
        assert_eq!(c.browsers.last().unwrap().name, "Firefox (Private)");
        c.validate().unwrap();

        // Running detection again changes nothing.
        let again = Browser {
            name: "Firefox (Private)".into(),
            args: vec!["--private-window".into()],
            ..Browser::new("Firefox", abs("/usr/bin/firefox"))
        };
        assert_eq!(c.merge_discovered(vec![again]), 0);
    }

    #[test]
    fn remember_domain_appends_or_creates() {
        let mut c = sample();
        let firefox = c.browsers[0].id;
        let chrome = c.browsers[1].id;
        c.remember_domain("news.example", chrome, None).unwrap();
        assert_eq!(c.rulesets.len(), 2);
        assert_eq!(
            c.rulesets[1].patterns,
            vec!["domain:*.news.example".parse::<Pattern>().unwrap()]
        );
        assert_eq!(c.rulesets[1].name, "Remembered: Chrome");
        c.remember_domain("news.example", chrome, None).unwrap();
        c.remember_domain("other.example", chrome, None).unwrap();
        assert_eq!(c.rulesets.len(), 2);
        assert_eq!(c.rulesets[1].patterns.len(), 2);
        // Different launch → different ruleset.
        c.remember_domain("x.example", firefox, None).unwrap();
        assert_eq!(c.rulesets.len(), 3);
        assert!(c.remember_domain("bad host", chrome, None).is_err());
        c.validate().unwrap();
    }

    #[test]
    fn merge_redirects_rules_to_existing_browsers() {
        let mut mine = sample();
        let firefox_id = mine.browsers[0].id;

        let mut theirs = Config::default();
        let their_launch = Launch {
            id: LaunchId::new(),
            name: "Work profile".into(),
            args: vec![],
        };
        let their_firefox = Browser {
            launches: vec![their_launch.clone()],
            ..Browser::new("FF", abs("/usr/bin/firefox"))
        };
        let their_brave = Browser::new("Brave", abs("/usr/bin/brave"));
        theirs.rulesets.push(Ruleset {
            name: "To FF".into(),
            browser: their_firefox.id,
            launch: Some(their_launch.id),
            patterns: vec!["domain:a.org".parse().unwrap()],
        });
        theirs.rulesets.push(Ruleset {
            name: "To Brave".into(),
            browser: their_brave.id,
            launch: None,
            patterns: vec!["domain:b.org".parse().unwrap()],
        });
        theirs.rulesets.push(Ruleset {
            name: "Dangling".into(),
            browser: BrowserId::new(),
            launch: None,
            patterns: vec![],
        });
        theirs.browsers = vec![their_firefox, their_brave];

        let report = mine.merge(theirs);
        assert_eq!((report.browsers_added, report.rulesets_added), (1, 2));
        assert_eq!(mine.browsers.len(), 3);
        assert_eq!(
            mine.rulesets[1].browser, firefox_id,
            "redirected to existing Firefox"
        );
        assert_eq!(mine.rulesets[1].launch, None, "foreign launch dropped");
        assert_eq!(mine.rulesets[2].name, "To Brave");
        assert_eq!(report.notes.len(), 3, "{:?}", report.notes);
        mine.validate().unwrap();
    }

    #[test]
    fn visible_browsers_filters_hidden() {
        let mut c = sample();
        c.browsers[1].hidden = true;
        assert_eq!(c.visible_browsers().count(), 1);
    }
}
