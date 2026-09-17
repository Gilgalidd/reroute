//! Recognise well-known browsers from their executable name.
//!
//! Signpost never downloads logos. When no icon file is available the UI
//! draws a coloured tile with the browser's initial; this module supplies a
//! stable key and an accent colour for the browsers people actually use, so
//! that "Firefox" is always orange and "Chrome" always blue.

use std::path::Path;

use serde::Serialize;

/// A recognised browser family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Brand {
    /// Stable lower-case key (`"firefox"`, `"chrome"`…), usable as a CSS class.
    pub key: &'static str,
    /// Accent colour as `#rrggbb`.
    pub color: &'static str,
}

/// Executable stem (lower-case, without extension) → brand. Order matters:
/// more specific names come first (`msedge` before `edge`-like matches are
/// not an issue since we compare whole stems, but `chromium` must not be
/// caught by a `chrome` prefix rule, hence exact stems).
const TABLE: &[(&str, Brand)] = &[
    (
        "firefox",
        Brand {
            key: "firefox",
            color: "#ff7139",
        },
    ),
    (
        "firefox-bin",
        Brand {
            key: "firefox",
            color: "#ff7139",
        },
    ),
    (
        "firefox-esr",
        Brand {
            key: "firefox",
            color: "#ff7139",
        },
    ),
    (
        "firefox-developer-edition",
        Brand {
            key: "firefox",
            color: "#1d6bbf",
        },
    ),
    (
        "firefox-nightly",
        Brand {
            key: "firefox",
            color: "#5b2ea6",
        },
    ),
    (
        "chrome",
        Brand {
            key: "chrome",
            color: "#4285f4",
        },
    ),
    (
        "google chrome",
        Brand {
            key: "chrome",
            color: "#4285f4",
        },
    ),
    (
        "google-chrome",
        Brand {
            key: "chrome",
            color: "#4285f4",
        },
    ),
    (
        "google-chrome-stable",
        Brand {
            key: "chrome",
            color: "#4285f4",
        },
    ),
    (
        "google-chrome-beta",
        Brand {
            key: "chrome",
            color: "#4285f4",
        },
    ),
    (
        "chromium",
        Brand {
            key: "chromium",
            color: "#4a8af4",
        },
    ),
    (
        "chromium-browser",
        Brand {
            key: "chromium",
            color: "#4a8af4",
        },
    ),
    (
        "msedge",
        Brand {
            key: "edge",
            color: "#0f7bd8",
        },
    ),
    (
        "microsoft edge",
        Brand {
            key: "edge",
            color: "#0f7bd8",
        },
    ),
    (
        "microsoft-edge",
        Brand {
            key: "edge",
            color: "#0f7bd8",
        },
    ),
    (
        "microsoft-edge-stable",
        Brand {
            key: "edge",
            color: "#0f7bd8",
        },
    ),
    (
        "brave",
        Brand {
            key: "brave",
            color: "#fb542b",
        },
    ),
    (
        "brave-browser",
        Brand {
            key: "brave",
            color: "#fb542b",
        },
    ),
    (
        "brave browser",
        Brand {
            key: "brave",
            color: "#fb542b",
        },
    ),
    (
        "opera",
        Brand {
            key: "opera",
            color: "#ff1b2d",
        },
    ),
    (
        "opera-gx",
        Brand {
            key: "opera",
            color: "#fa1e4e",
        },
    ),
    (
        "vivaldi",
        Brand {
            key: "vivaldi",
            color: "#ef3939",
        },
    ),
    (
        "vivaldi-stable",
        Brand {
            key: "vivaldi",
            color: "#ef3939",
        },
    ),
    (
        "safari",
        Brand {
            key: "safari",
            color: "#006cff",
        },
    ),
    (
        "arc",
        Brand {
            key: "arc",
            color: "#ff536a",
        },
    ),
    (
        "zen",
        Brand {
            key: "zen",
            color: "#1c1c1c",
        },
    ),
    (
        "zen-browser",
        Brand {
            key: "zen",
            color: "#1c1c1c",
        },
    ),
    (
        "librewolf",
        Brand {
            key: "librewolf",
            color: "#00acff",
        },
    ),
    (
        "waterfox",
        Brand {
            key: "waterfox",
            color: "#0d4b8c",
        },
    ),
    (
        "tor browser",
        Brand {
            key: "tor",
            color: "#7d4698",
        },
    ),
    (
        "torbrowser",
        Brand {
            key: "tor",
            color: "#7d4698",
        },
    ),
    (
        "mullvad browser",
        Brand {
            key: "mullvad",
            color: "#294d73",
        },
    ),
    (
        "mullvad-browser",
        Brand {
            key: "mullvad",
            color: "#294d73",
        },
    ),
    (
        "epiphany",
        Brand {
            key: "epiphany",
            color: "#3584e4",
        },
    ),
    (
        "konqueror",
        Brand {
            key: "konqueror",
            color: "#1d99f3",
        },
    ),
    (
        "falkon",
        Brand {
            key: "falkon",
            color: "#2c6bbd",
        },
    ),
    (
        "floorp",
        Brand {
            key: "floorp",
            color: "#0a84ff",
        },
    ),
    (
        "thorium",
        Brand {
            key: "thorium",
            color: "#3b7dd8",
        },
    ),
];

impl Brand {
    /// Recognise the browser from the executable path (its file stem,
    /// case-insensitively). Both `/` and `\\` are accepted as separators so
    /// that imported Windows paths are recognised on any OS. Returns `None`
    /// for unknown programs.
    pub fn from_program(path: &Path) -> Option<Self> {
        let text = path.to_str()?;
        let file = text.rsplit(['/', '\\']).next()?;
        let stem = file
            .rsplit_once('.')
            .map_or(file, |(s, _)| s)
            .to_ascii_lowercase();
        TABLE
            .iter()
            .find(|(name, _)| *name == stem)
            .map(|(_, brand)| *brand)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_common_executables() {
        assert_eq!(
            Brand::from_program(Path::new("/usr/bin/firefox"))
                .unwrap()
                .key,
            "firefox"
        );
        assert_eq!(
            Brand::from_program(Path::new(
                r"C:\Program Files\Google\Chrome\Application\chrome.exe"
            ))
            .unwrap()
            .key,
            "chrome"
        );
        assert_eq!(
            Brand::from_program(Path::new("/Applications/Safari.app/Contents/MacOS/Safari"))
                .unwrap()
                .key,
            "safari"
        );
        assert_eq!(
            Brand::from_program(Path::new("/usr/bin/chromium"))
                .unwrap()
                .key,
            "chromium"
        );
        assert_eq!(
            Brand::from_program(Path::new("C:/x/MSEDGE.EXE"))
                .unwrap()
                .key,
            "edge"
        );
    }

    #[test]
    fn unknown_programs_yield_none() {
        assert!(Brand::from_program(Path::new("/usr/bin/env")).is_none());
        assert!(Brand::from_program(Path::new("/usr/bin/chrome-wrapper-thing")).is_none());
        assert!(Brand::from_program(Path::new("")).is_none());
    }

    #[test]
    fn colours_are_hex() {
        for (_, b) in TABLE {
            assert!(
                b.color.len() == 7 && b.color.starts_with('#'),
                "{}",
                b.color
            );
        }
    }
}
