//! Find the browsers installed on this machine.
//!
//! Discovery only *proposes* entries; the user's configuration is the
//! source of truth and is merged with [`reroute_core::Config::merge_discovered`],
//! which never overwrites an existing entry.

use std::path::{Path, PathBuf};

use reroute_core::{Brand, Browser};

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(windows)]
pub mod windows;

/// Browsers registered with the operating system as HTTP handlers, in a
/// stable order (alphabetical by name). Reroute itself is excluded.
pub fn installed_browsers() -> Vec<Browser> {
    #[cfg(target_os = "linux")]
    let mut found = linux::installed_browsers();
    #[cfg(target_os = "macos")]
    let mut found = macos::installed_browsers();
    #[cfg(windows)]
    let mut found = windows::installed_browsers();
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    let mut found: Vec<Browser> = Vec::new();

    found.retain(|b| !is_self(&b.path));
    found.sort_by_key(|b| b.name.to_lowercase());
    found.dedup_by(|a, b| a.path == b.path);
    with_private_entries(found)
}

/// Put a private entry after each browser that has one, so that the picker
/// offers both the ordinary window and the private one.
fn with_private_entries(browsers: Vec<Browser>) -> Vec<Browser> {
    let mut all = Vec::with_capacity(browsers.len());
    for browser in browsers {
        let private = Brand::from_program(&browser.path)
            .and_then(|brand| reroute_core::private::private_entry(&browser, brand.key));
        all.push(browser);
        all.extend(private);
    }
    all
}

/// Is `path` this very program? Guards against listing Reroute as a
/// browser, which would loop forever.
fn is_self(path: &Path) -> bool {
    let me = std::env::current_exe()
        .ok()
        .and_then(|p| p.canonicalize().ok());
    let them = path.canonicalize().ok();
    if me.is_some() && me == them {
        return true;
    }
    // Compare the file name on either separator so that the check also
    // holds for paths imported from another OS.
    let text = path.to_string_lossy();
    let file = text.rsplit(['/', '\\']).next().unwrap_or_default();
    let stem = file.rsplit_once('.').map_or(file, |(s, _)| s);
    stem.eq_ignore_ascii_case("reroute")
}

/// Keep only absolute, existing paths; used by every backend.
pub(crate) fn existing_absolute(path: PathBuf) -> Option<PathBuf> {
    (path.is_absolute() && path.is_file()).then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reroute_is_never_a_browser() {
        assert!(is_self(Path::new("/opt/reroute/reroute")));
        assert!(is_self(Path::new(r"C:\Program Files\Reroute\Reroute.exe")));
        assert!(!is_self(Path::new("/usr/bin/firefox")));
    }

    #[test]
    fn a_private_entry_follows_each_browser_that_has_one() {
        // Discovery gives every browser the arguments of its desktop entry.
        let with_url = |name: &str, path: &str| Browser {
            args: vec!["%URL%".to_owned()],
            ..Browser::new(name, path)
        };
        let browsers = vec![
            with_url("Firefox", "/usr/bin/firefox"),
            with_url("Konqueror", "/usr/bin/konqueror"),
            with_url("Chromium", "/usr/bin/chromium"),
        ];
        let all = with_private_entries(browsers);
        let names: Vec<&str> = all.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Firefox",
                "Firefox (Private)",
                // Reroute knows no argument for Konqueror, so it stays alone.
                "Konqueror",
                "Chromium",
                "Chromium (Incognito)",
            ]
        );
        assert_eq!(
            all[1].path, all[0].path,
            "the same program, started differently"
        );
        assert_eq!(all[1].args, ["--private-window", "%URL%"]);
    }

    #[test]
    fn existing_absolute_filters() {
        assert!(existing_absolute(PathBuf::from("relative")).is_none());
        assert!(existing_absolute(PathBuf::from("/definitely/missing")).is_none());
        let dir = tempfile::tempdir().unwrap();
        assert!(existing_absolute(dir.path().to_path_buf()).is_none());
        let f = dir.path().join("f");
        std::fs::write(&f, b"").unwrap();
        assert_eq!(existing_absolute(f.clone()), Some(f));
    }
}
