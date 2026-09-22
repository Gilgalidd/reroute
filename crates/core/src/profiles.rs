//! Reading a browser's own configuration to offer its profiles and its
//! private-browsing window as launch options.
//!
//! Browsers keep the list of their profiles in a file of their own: an INI
//! file for the Firefox family, a JSON file for the Chromium family. This
//! module knows where those files are and how to read them; opening them is
//! left to `reroute_platform::discover`.
//!
//! Only names are taken. Nothing else of a browser profile is read, copied
//! or sent anywhere.

use std::path::{Path, PathBuf};

use crate::browser::Launch;

/// Which engine a browser is built on, which decides both where its
/// profiles are listed and the arguments that select one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// Firefox and its derivatives: `profiles.ini`, selected with `-P`.
    Firefox,
    /// Chrome and its derivatives: `Local State`, selected with
    /// `--profile-directory`.
    Chromium,
}

/// A profile a browser already has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    /// Name the browser shows for it.
    pub name: String,
    /// What to pass on the command line to select it.
    pub selector: String,
}

/// The engine behind a [`crate::Brand`] key, when Reroute knows it.
pub fn family(brand: &str) -> Option<Family> {
    match brand {
        "firefox" | "librewolf" | "waterfox" | "floorp" | "zen" => Some(Family::Firefox),
        "chrome" | "chromium" | "edge" | "brave" | "vivaldi" | "thorium" => Some(Family::Chromium),
        _ => None,
    }
}

/// How this browser opens a window that keeps no history, as a label and
/// the argument that asks for it.
pub fn private_window(brand: &str) -> Option<(&'static str, &'static str)> {
    match brand {
        "firefox" | "librewolf" | "waterfox" | "floorp" | "zen" | "tor" | "mullvad" => {
            Some(("Private window", "--private-window"))
        }
        "chrome" | "chromium" | "brave" | "vivaldi" | "thorium" => {
            Some(("Incognito window", "--incognito"))
        }
        "edge" => Some(("InPrivate window", "--inprivate")),
        "opera" => Some(("Private window", "--private")),
        "epiphany" => Some(("Private window", "--incognito-mode")),
        "falkon" => Some(("Private window", "--private-browsing")),
        _ => None,
    }
}

/// Files that may list this browser's profiles, best first. `home` is the
/// user's home directory and `os` is [`std::env::consts::OS`].
pub fn profile_files(brand: &str, home: &Path, os: &str) -> Vec<PathBuf> {
    let (dirs, file) = match family(brand) {
        Some(Family::Firefox) => (firefox_dirs(brand, os), "profiles.ini"),
        Some(Family::Chromium) => (chromium_dirs(brand, os), "Local State"),
        None => return Vec::new(),
    };
    dirs.into_iter()
        .map(|dir| home.join(dir).join(file))
        .collect()
}

fn firefox_dirs(brand: &str, os: &str) -> Vec<&'static str> {
    match (brand, os) {
        ("firefox", "linux") => vec![
            ".mozilla/firefox",
            "snap/firefox/common/.mozilla/firefox",
            ".var/app/org.mozilla.firefox/.mozilla/firefox",
        ],
        ("firefox", "macos") => vec!["Library/Application Support/Firefox"],
        ("firefox", "windows") => vec!["AppData/Roaming/Mozilla/Firefox"],
        ("librewolf", "linux") => {
            vec![
                ".librewolf",
                ".var/app/io.gitlab.librewolf-community/.librewolf",
            ]
        }
        ("librewolf", "macos") => vec!["Library/Application Support/librewolf"],
        ("librewolf", "windows") => vec!["AppData/Roaming/librewolf"],
        ("waterfox", "linux") => vec![".waterfox"],
        ("waterfox", "macos") => vec!["Library/Application Support/Waterfox"],
        ("waterfox", "windows") => vec!["AppData/Roaming/Waterfox"],
        ("floorp", "linux") => vec![".floorp", ".var/app/one.ablaze.floorp/.floorp"],
        ("floorp", "macos") => vec!["Library/Application Support/Floorp"],
        ("floorp", "windows") => vec!["AppData/Roaming/Floorp"],
        ("zen", "linux") => vec![".zen", ".var/app/app.zen_browser.zen/.zen"],
        ("zen", "macos") => vec!["Library/Application Support/zen"],
        ("zen", "windows") => vec!["AppData/Roaming/zen"],
        _ => Vec::new(),
    }
}

fn chromium_dirs(brand: &str, os: &str) -> Vec<&'static str> {
    match (brand, os) {
        ("chrome", "linux") => vec![".config/google-chrome", ".config/google-chrome-beta"],
        ("chrome", "macos") => vec!["Library/Application Support/Google/Chrome"],
        ("chrome", "windows") => vec!["AppData/Local/Google/Chrome/User Data"],
        ("chromium", "linux") => vec![
            ".config/chromium",
            "snap/chromium/common/chromium",
            ".var/app/org.chromium.Chromium/config/chromium",
        ],
        ("chromium", "macos") => vec!["Library/Application Support/Chromium"],
        ("chromium", "windows") => vec!["AppData/Local/Chromium/User Data"],
        ("edge", "linux") => vec![".config/microsoft-edge"],
        ("edge", "macos") => vec!["Library/Application Support/Microsoft Edge"],
        ("edge", "windows") => vec!["AppData/Local/Microsoft/Edge/User Data"],
        ("brave", "linux") => vec![
            ".config/BraveSoftware/Brave-Browser",
            ".var/app/com.brave.Browser/config/BraveSoftware/Brave-Browser",
        ],
        ("brave", "macos") => vec!["Library/Application Support/BraveSoftware/Brave-Browser"],
        ("brave", "windows") => vec!["AppData/Local/BraveSoftware/Brave-Browser/User Data"],
        ("vivaldi", "linux") => vec![".config/vivaldi"],
        ("vivaldi", "macos") => vec!["Library/Application Support/Vivaldi"],
        ("vivaldi", "windows") => vec!["AppData/Local/Vivaldi/User Data"],
        ("thorium", "linux") => vec![".config/thorium"],
        ("thorium", "windows") => vec!["AppData/Local/Thorium/User Data"],
        _ => Vec::new(),
    }
}

/// Read the profiles out of the file named by [`profile_files`].
pub fn parse(brand: &str, text: &str) -> Vec<Profile> {
    match family(brand) {
        Some(Family::Firefox) => parse_profiles_ini(text),
        Some(Family::Chromium) => parse_local_state(text),
        None => Vec::new(),
    }
}

/// Firefox's `profiles.ini`: one `[ProfileN]` section per profile, each
/// with a `Name`. Other sections (`[General]`, `[InstallXXXX]`) are skipped.
pub fn parse_profiles_ini(text: &str) -> Vec<Profile> {
    let mut profiles = Vec::new();
    let mut in_profile = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_profile = line
                .strip_prefix("[Profile")
                .is_some_and(|rest| rest.ends_with(']'));
            continue;
        }
        if !in_profile {
            continue;
        }
        if let Some(name) = line.strip_prefix("Name=") {
            let name = name.trim();
            if !name.is_empty() {
                profiles.push(Profile {
                    name: name.to_owned(),
                    selector: name.to_owned(),
                });
            }
        }
    }
    profiles
}

/// Chromium's `Local State`: `profile.info_cache` maps a directory to the
/// profile's settings, and `profile.profiles_order` gives the order the
/// browser shows them in.
pub fn parse_local_state(json: &str) -> Vec<Profile> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    let Some(cache) = value
        .pointer("/profile/info_cache")
        .and_then(|c| c.as_object())
    else {
        return Vec::new();
    };

    let mut directories: Vec<&String> = cache.keys().collect();
    if let Some(order) = value
        .pointer("/profile/profiles_order")
        .and_then(|o| o.as_array())
    {
        let wanted: Vec<&str> = order.iter().filter_map(serde_json::Value::as_str).collect();
        directories.sort_by_key(|dir| wanted.iter().position(|w| w == dir).unwrap_or(usize::MAX));
    }

    directories
        .into_iter()
        .filter_map(|dir| {
            let name = cache.get(dir)?.get("name")?.as_str()?.trim();
            let name = if name.is_empty() { dir.as_str() } else { name };
            Some(Profile {
                name: name.to_owned(),
                selector: dir.clone(),
            })
        })
        .collect()
}

/// Launch options to offer for a browser: its private window, and one entry
/// per profile once there are several. A lone profile is the one that opens
/// anyway, so offering it would only add noise.
pub fn launches(brand: &str, profiles: &[Profile]) -> Vec<Launch> {
    let mut launches = Vec::new();
    if let Some((label, flag)) = private_window(brand) {
        launches.push(Launch {
            id: crate::browser::LaunchId::new(),
            name: label.to_owned(),
            args: vec![flag.to_owned(), crate::URL_PLACEHOLDER.to_owned()],
        });
    }
    if profiles.len() > 1 {
        for profile in profiles {
            let args = match family(brand) {
                Some(Family::Firefox) => vec![
                    "-P".to_owned(),
                    profile.selector.clone(),
                    crate::URL_PLACEHOLDER.to_owned(),
                ],
                Some(Family::Chromium) => vec![
                    format!("--profile-directory={}", profile.selector),
                    crate::URL_PLACEHOLDER.to_owned(),
                ],
                None => continue,
            };
            launches.push(Launch {
                id: crate::browser::LaunchId::new(),
                name: format!("Profile: {}", profile.name),
                args,
            });
        }
    }
    launches
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIREFOX_INI: &str = "[Install4F96D1932A9F858E]\nDefault=abc.default-release\nLocked=1\n\n[Profile1]\nName=Work\nIsRelative=1\nPath=w0rk.Work\n\n[Profile0]\nName=default-release\nIsRelative=1\nPath=abc.default-release\nDefault=1\n\n[General]\nStartWithLastProfile=1\nVersion=2\n";

    const LOCAL_STATE: &str = r#"{"profile":{"info_cache":{
        "Profile 1":{"name":"Work","active_time":1.0},
        "Default":{"name":"Personal"},
        "Profile 3":{"name":""}},
        "profiles_order":["Default","Profile 1","Profile 3"]},"other":{"x":1}}"#;

    #[test]
    fn reads_firefox_profiles_and_ignores_other_sections() {
        let profiles = parse_profiles_ini(FIREFOX_INI);
        assert_eq!(
            profiles,
            vec![
                Profile {
                    name: "Work".into(),
                    selector: "Work".into()
                },
                Profile {
                    name: "default-release".into(),
                    selector: "default-release".into()
                },
            ]
        );
        assert!(parse_profiles_ini("[General]\nName=not a profile\n").is_empty());
        assert!(parse_profiles_ini("").is_empty());
    }

    #[test]
    fn reads_chromium_profiles_in_the_browsers_own_order() {
        let profiles = parse_local_state(LOCAL_STATE);
        assert_eq!(
            profiles,
            vec![
                Profile {
                    name: "Personal".into(),
                    selector: "Default".into()
                },
                Profile {
                    name: "Work".into(),
                    selector: "Profile 1".into()
                },
                // An unnamed profile falls back to its directory.
                Profile {
                    name: "Profile 3".into(),
                    selector: "Profile 3".into()
                },
            ]
        );
        assert!(parse_local_state("not json").is_empty());
        assert!(parse_local_state(r#"{"profile":{}}"#).is_empty());
    }

    #[test]
    fn a_single_profile_adds_only_the_private_window() {
        let one = vec![Profile {
            name: "default".into(),
            selector: "default".into(),
        }];
        let made = launches("firefox", &one);
        assert_eq!(made.len(), 1);
        assert_eq!(made[0].name, "Private window");
        assert_eq!(made[0].args, vec!["--private-window", "%URL%"]);
    }

    #[test]
    fn several_profiles_each_get_an_entry() {
        let profiles = parse_local_state(LOCAL_STATE);
        let made = launches("chrome", &profiles);
        let names: Vec<&str> = made.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Incognito window",
                "Profile: Personal",
                "Profile: Work",
                "Profile: Profile 3"
            ]
        );
        assert_eq!(made[1].args, vec!["--profile-directory=Default", "%URL%"]);

        let firefox = launches("firefox", &parse_profiles_ini(FIREFOX_INI));
        assert_eq!(firefox[1].args, vec!["-P", "Work", "%URL%"]);
        assert!(made
            .iter()
            .all(|l| l.args.last().is_some_and(|a| a == "%URL%")));
    }

    #[test]
    fn private_browsing_differs_between_browsers() {
        assert_eq!(
            private_window("edge"),
            Some(("InPrivate window", "--inprivate"))
        );
        assert_eq!(
            private_window("opera"),
            Some(("Private window", "--private"))
        );
        assert_eq!(
            private_window("brave"),
            Some(("Incognito window", "--incognito"))
        );
        assert_eq!(
            private_window("safari"),
            None,
            "no reliable flag, so nothing is offered"
        );
        assert!(launches("safari", &[]).is_empty());
    }

    #[test]
    fn profile_files_follow_each_platform() {
        let home = Path::new("/home/me");
        let firefox = profile_files("firefox", home, "linux");
        assert_eq!(
            firefox[0],
            Path::new("/home/me/.mozilla/firefox/profiles.ini")
        );
        assert!(firefox.contains(&PathBuf::from(
            "/home/me/snap/firefox/common/.mozilla/firefox/profiles.ini"
        )));

        let chromium = profile_files("chromium", home, "linux");
        assert!(chromium.contains(&PathBuf::from(
            "/home/me/snap/chromium/common/chromium/Local State"
        )));

        assert_eq!(
            profile_files("chrome", Path::new("/Users/me"), "macos"),
            vec![PathBuf::from(
                "/Users/me/Library/Application Support/Google/Chrome/Local State"
            )]
        );
        assert_eq!(
            profile_files("firefox", Path::new(r"C:\Users\me"), "windows")[0],
            PathBuf::from(r"C:\Users\me").join("AppData/Roaming/Mozilla/Firefox/profiles.ini")
        );
        assert!(
            profile_files("safari", home, "macos").is_empty(),
            "unknown family, no file"
        );
        assert!(profile_files("firefox", home, "freebsd").is_empty());
    }

    #[test]
    fn parse_dispatches_on_the_family() {
        assert_eq!(parse("firefox", FIREFOX_INI).len(), 2);
        assert_eq!(parse("chrome", LOCAL_STATE).len(), 3);
        assert!(parse("safari", LOCAL_STATE).is_empty());
    }
}
