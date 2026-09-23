//! Opening a browser in a window that keeps no history.
//!
//! Every mainstream browser has an argument for this, but each family
//! spells it differently. Reroute offers the option only for browsers it
//! knows the argument of, rather than guessing and producing a launch that
//! silently opens an ordinary window.

use crate::browser::{Launch, LaunchId};

/// How this browser opens a private window, as a label to show and the
/// argument that asks for it. `None` when Reroute knows no reliable way.
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

/// The launch options detection offers for a browser: its private window,
/// when it has one. Profiles are left to the user, who knows which of them
/// are worth a place in the picker.
pub fn launches(brand: &str) -> Vec<Launch> {
    private_window(brand)
        .into_iter()
        .map(|(label, flag)| Launch {
            id: LaunchId::new(),
            name: label.to_owned(),
            args: vec![flag.to_owned(), crate::URL_PLACEHOLDER.to_owned()],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_family_gets_its_own_argument() {
        assert_eq!(
            private_window("firefox"),
            Some(("Private window", "--private-window"))
        );
        assert_eq!(
            private_window("chrome"),
            Some(("Incognito window", "--incognito"))
        );
        assert_eq!(
            private_window("edge"),
            Some(("InPrivate window", "--inprivate"))
        );
        assert_eq!(
            private_window("opera"),
            Some(("Private window", "--private"))
        );
    }

    #[test]
    fn a_browser_with_no_known_argument_is_offered_nothing() {
        assert_eq!(private_window("safari"), None);
        assert_eq!(private_window("konqueror"), None);
        assert!(launches("safari").is_empty());
    }

    #[test]
    fn the_option_carries_the_url_after_the_flag() {
        let made = launches("firefox");
        assert_eq!(made.len(), 1);
        assert_eq!(made[0].name, "Private window");
        assert_eq!(made[0].args, ["--private-window", "%URL%"]);

        let chromium = launches("chromium");
        assert_eq!(chromium[0].args, ["--incognito", "%URL%"]);
        // Two calls never share an id.
        assert_ne!(made[0].id, launches("firefox")[0].id);
    }
}
