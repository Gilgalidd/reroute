//! Opening a browser in a window that keeps no history.
//!
//! Every mainstream browser has an argument for this, but each family
//! spells it differently. Detection turns it into a second entry beside the
//! browser, so that a private window is one click away in the picker rather
//! than hidden behind a menu. A browser whose argument Reroute does not
//! know gets its ordinary entry and nothing else.

use crate::browser::{Browser, BrowserId};

/// How a browser opens a window that keeps no history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrivateMode {
    /// The word this browser uses for it, shown in the entry's name.
    pub word: &'static str,
    /// The argument that asks for it.
    pub flag: &'static str,
}

/// The private mode of a browser, or `None` when Reroute knows no reliable
/// argument for it.
pub fn private_mode(brand: &str) -> Option<PrivateMode> {
    let (word, flag) = match brand {
        "firefox" | "librewolf" | "waterfox" | "floorp" | "zen" | "tor" | "mullvad" => {
            ("Private", "--private-window")
        }
        "chrome" | "chromium" | "brave" | "vivaldi" | "thorium" => ("Incognito", "--incognito"),
        "edge" => ("InPrivate", "--inprivate"),
        "opera" => ("Private", "--private"),
        "epiphany" => ("Private", "--incognito-mode"),
        "falkon" => ("Private", "--private-browsing"),
        _ => return None,
    };
    Some(PrivateMode { word, flag })
}

/// The second entry to offer beside `browser`: the same program, started so
/// that it keeps no history. `None` when this browser has no known
/// argument for that.
pub fn private_entry(browser: &Browser, brand: &str) -> Option<Browser> {
    let mode = private_mode(brand)?;
    Some(Browser {
        id: BrowserId::new(),
        name: format!("{} ({})", browser.name, mode.word),
        args: with_flag(&browser.args, mode.flag),
        launches: Vec::new(),
        ..browser.clone()
    })
}

/// Add `flag` to an argument list, before the argument that carries the
/// URL. A browser started through a wrapper, such as `flatpak run …`, keeps
/// the arguments that reach it.
fn with_flag(args: &[String], flag: &str) -> Vec<String> {
    let mut args = args.to_vec();
    let at = args
        .iter()
        .position(|a| a.contains(crate::URL_PLACEHOLDER))
        .unwrap_or(args.len());
    args.insert(at, flag.to_owned());
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    fn firefox() -> Browser {
        let mut browser = Browser::new("Firefox", "/usr/bin/firefox");
        browser.args = vec!["%URL%".to_owned()];
        browser.icon = Some("/icons/firefox.png".into());
        browser
    }

    #[test]
    fn each_family_has_its_own_word_and_argument() {
        assert_eq!(private_mode("firefox").unwrap().flag, "--private-window");
        assert_eq!(private_mode("chrome").unwrap().word, "Incognito");
        assert_eq!(
            private_mode("edge").unwrap(),
            PrivateMode {
                word: "InPrivate",
                flag: "--inprivate"
            }
        );
        assert_eq!(private_mode("opera").unwrap().flag, "--private");
    }

    #[test]
    fn a_browser_with_no_known_argument_gets_no_second_entry() {
        assert_eq!(private_mode("safari"), None);
        assert!(private_entry(&Browser::new("Safari", "/Applications/Safari"), "safari").is_none());
    }

    #[test]
    fn the_second_entry_is_the_same_browser_started_differently() {
        let plain = firefox();
        let private = private_entry(&plain, "firefox").unwrap();
        assert_eq!(private.name, "Firefox (Private)");
        assert_eq!(private.path, plain.path);
        assert_eq!(private.icon, plain.icon);
        assert_eq!(private.args, ["--private-window", "%URL%"]);
        assert_ne!(private.id, plain.id, "rules can tell them apart");
        assert!(private.launches.is_empty());
    }

    #[test]
    fn a_wrapped_browser_keeps_the_arguments_that_reach_it() {
        let mut flatpak = Browser::new("Firefox", "/usr/bin/flatpak");
        flatpak.args = ["run", "org.mozilla.firefox", "%URL%"]
            .map(String::from)
            .to_vec();
        let private = private_entry(&flatpak, "firefox").unwrap();
        assert_eq!(
            private.args,
            ["run", "org.mozilla.firefox", "--private-window", "%URL%"]
        );

        // The placeholder may be part of an argument.
        let mut app = firefox();
        app.args = vec!["--app=%URL%".to_owned()];
        assert_eq!(
            private_entry(&app, "firefox").unwrap().args,
            ["--private-window", "--app=%URL%"]
        );

        // With no placeholder at all the flag goes last, and the URL is
        // appended after it when the browser is started.
        let mut bare = firefox();
        bare.args = Vec::new();
        assert_eq!(
            private_entry(&bare, "firefox").unwrap().args,
            ["--private-window"]
        );
    }
}
