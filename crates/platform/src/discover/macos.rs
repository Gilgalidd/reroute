//! macOS: scan application bundles whose `Info.plist` declares `http` or
//! `https` in `CFBundleURLTypes`.

use std::path::{Path, PathBuf};

use reroute_core::Browser;

/// The parts of an `Info.plist` we care about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleInfo {
    /// `CFBundleDisplayName`, else `CFBundleName`.
    pub name: String,
    /// `CFBundleExecutable` (file name inside `Contents/MacOS`).
    pub executable: String,
    /// `CFBundleIconFile`, with `.icns` appended when missing.
    pub icon_file: Option<String>,
    /// Whether any `CFBundleURLTypes` entry lists `http` or `https`.
    pub handles_http: bool,
}

/// Extract [`BundleInfo`] from a parsed plist dictionary.
#[cfg(target_os = "macos")]
pub fn bundle_info(plist: &plist::Value) -> Option<BundleInfo> {
    let dict = plist.as_dictionary()?;
    let string = |key: &str| {
        dict.get(key)
            .and_then(plist::Value::as_string)
            .map(str::to_owned)
    };
    let name = string("CFBundleDisplayName").or_else(|| string("CFBundleName"))?;
    let executable = string("CFBundleExecutable")?;
    let icon_file = string("CFBundleIconFile").map(|f| {
        let has_ext = Path::new(&f)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("icns"));
        if has_ext { f } else { format!("{f}.icns") }
    });
    let handles_http = dict
        .get("CFBundleURLTypes")
        .and_then(plist::Value::as_array)
        .is_some_and(|types| {
            types.iter().any(|t| {
                t.as_dictionary()
                    .and_then(|d| d.get("CFBundleURLSchemes"))
                    .and_then(plist::Value::as_array)
                    .is_some_and(|schemes| {
                        schemes
                            .iter()
                            .any(|s| matches!(s.as_string(), Some("http" | "https")))
                    })
            })
        });
    Some(BundleInfo {
        name,
        executable,
        icon_file,
        handles_http,
    })
}

/// Convert one `.app` directory into a browser, if it handles web links.
#[cfg(target_os = "macos")]
fn browser_from_bundle(app: &Path) -> Option<Browser> {
    let plist = plist::Value::from_file(app.join("Contents/Info.plist")).ok()?;
    let info = bundle_info(&plist)?;
    if !info.handles_http {
        return None;
    }
    let executable = super::existing_absolute(app.join("Contents/MacOS").join(&info.executable))?;
    let mut browser = Browser::new(info.name, executable);
    browser.icon = info
        .icon_file
        .map(|f| app.join("Contents/Resources").join(f))
        .and_then(super::existing_absolute);
    Some(browser)
}

/// Folders scanned for bundles.
fn application_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(Path::new(&home).join("Applications"));
    }
    dirs
}

/// Enumerate bundles in the standard application folders.
#[cfg(target_os = "macos")]
pub fn installed_browsers() -> Vec<Browser> {
    let mut browsers = Vec::new();
    for dir in application_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "app") {
                if let Some(b) = browser_from_bundle(&path) {
                    browsers.push(b);
                }
            }
        }
    }
    browsers
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
pub fn installed_browsers() -> Vec<Browser> {
    let _ = application_dirs();
    Vec::new()
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    const SAFARI: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>Safari</string>
  <key>CFBundleExecutable</key><string>Safari</string>
  <key>CFBundleIconFile</key><string>compass</string>
  <key>CFBundleURLTypes</key><array>
    <dict><key>CFBundleURLName</key><string>Web site URL</string>
      <key>CFBundleURLSchemes</key><array><string>http</string><string>https</string></array></dict>
  </array>
</dict></plist>"#;

    #[test]
    fn extracts_bundle_info() {
        let value = plist::Value::from_reader_xml(SAFARI.as_bytes()).unwrap();
        let info = bundle_info(&value).unwrap();
        assert_eq!(info.name, "Safari");
        assert_eq!(info.executable, "Safari");
        assert_eq!(info.icon_file.as_deref(), Some("compass.icns"));
        assert!(info.handles_http);
    }

    #[test]
    fn non_browser_bundle_is_flagged() {
        let text = SAFARI.replace(
            "<string>http</string><string>https</string>",
            "<string>ftp</string>",
        );
        let value = plist::Value::from_reader_xml(text.as_bytes()).unwrap();
        assert!(!bundle_info(&value).unwrap().handles_http);
    }
}
