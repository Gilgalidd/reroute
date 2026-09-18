//! Linux: parse freedesktop `.desktop` entries that declare themselves as
//! `x-scheme-handler/http(s)` handlers.
//!
//! Spec: <https://specifications.freedesktop.org/desktop-entry-spec/latest/>

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use signpost_core::Browser;

/// The parts of a desktop entry we care about.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DesktopEntry {
    /// `Name=`
    pub name: String,
    /// `Exec=` verbatim.
    pub exec: String,
    /// `Icon=` (a theme name or an absolute path).
    pub icon: Option<String>,
    /// `MimeType=` split on `;`.
    pub mime_types: Vec<String>,
    /// `NoDisplay=true` or `Hidden=true`.
    pub hidden: bool,
    /// `OnlyShowIn=` split on `;` (empty when absent).
    pub only_show_in: Vec<String>,
    /// `NotShowIn=` split on `;` (empty when absent).
    pub not_show_in: Vec<String>,
}

impl DesktopEntry {
    /// Does the entry handle web links?
    pub fn handles_http(&self) -> bool {
        self.mime_types
            .iter()
            .any(|m| m == "x-scheme-handler/https" || m == "x-scheme-handler/http")
    }

    /// Should the entry be offered on the given desktop environment
    /// (`XDG_CURRENT_DESKTOP`, colon-separated)? Entries restricted to another
    /// environment (e.g. a snap's `OnlyShowIn=UbuntuFrame;` stub whose `Exec`
    /// is `/usr/bin/false`) are not real browsers for this session.
    pub fn shown_on(&self, current_desktop: &str) -> bool {
        if self.hidden {
            return false;
        }
        let current: Vec<&str> = current_desktop
            .split(':')
            .filter(|d| !d.is_empty())
            .collect();
        let listed = |d: &String| current.contains(&d.as_str());
        if !self.only_show_in.is_empty() && !self.only_show_in.iter().any(listed) {
            return false;
        }
        !self.not_show_in.iter().any(listed)
    }
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(';')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Parse the `[Desktop Entry]` group of a `.desktop` file. Returns `None`
/// when the file is not an application entry.
pub fn parse_desktop_entry(text: &str) -> Option<DesktopEntry> {
    let mut in_group = false;
    let mut entry = DesktopEntry::default();
    let mut is_application = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            in_group = line == "[Desktop Entry]";
            continue;
        }
        if !in_group {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        match key {
            "Type" => is_application = value == "Application",
            "Name" => value.clone_into(&mut entry.name),
            "Exec" => value.clone_into(&mut entry.exec),
            "Icon" => entry.icon = Some(value.to_owned()),
            "MimeType" => entry.mime_types = split_list(value),
            "OnlyShowIn" => entry.only_show_in = split_list(value),
            "NotShowIn" => entry.not_show_in = split_list(value),
            "NoDisplay" | "Hidden" if value.eq_ignore_ascii_case("true") => entry.hidden = true,
            _ => {}
        }
    }
    (is_application && !entry.name.is_empty() && !entry.exec.is_empty()).then_some(entry)
}

/// Split an `Exec=` line into program and arguments.
///
/// Handles the spec's quoting (double quotes, backslash escapes), removes
/// field codes we cannot fill (`%f`, `%i`, `%c`…), and turns `%u`/`%U`
/// into Signpost's [`signpost_core::URL_PLACEHOLDER`].
pub fn split_exec(exec: &str) -> Option<(String, Vec<String>)> {
    let words = tokenize(exec)?;
    let mut program = None;
    let mut args = Vec::new();
    for word in words {
        let word = match word.as_str() {
            "%u" | "%U" | "%f" | "%F" => signpost_core::URL_PLACEHOLDER.to_owned(),
            "%d" | "%D" | "%n" | "%N" | "%i" | "%c" | "%k" | "%v" | "%m" => continue,
            _ => word.replace("%%", "%"),
        };
        if program.is_none() {
            program = Some(word);
        } else {
            args.push(word);
        }
    }
    program.map(|p| (p, args))
}

/// Desktop-entry tokenizer: whitespace separates words, double quotes group
/// them and `\` escapes the next character inside quotes.
fn tokenize(exec: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = exec.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => in_quotes = !in_quotes,
            '\\' if in_quotes => current.push(chars.next()?),
            c if c.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if in_quotes {
        return None;
    }
    if !current.is_empty() {
        words.push(current);
    }
    Some(words)
}

/// Resolve a bare program name through `PATH`.
fn resolve_program(name: &str) -> Option<PathBuf> {
    let path = Path::new(name);
    if path.is_absolute() {
        return super::existing_absolute(path.to_path_buf());
    }
    std::env::var_os("PATH")?
        .to_str()?
        .split(':')
        .filter(|d| !d.is_empty())
        .map(|d| Path::new(d).join(name))
        .find_map(super::existing_absolute)
}

/// Directories containing `applications/`, in precedence order (user first).
fn data_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut dirs = Vec::new();
    match std::env::var_os("XDG_DATA_HOME") {
        Some(d) => dirs.push(PathBuf::from(d)),
        None => {
            if let Some(h) = &home {
                dirs.push(h.join(".local/share"));
            }
        }
    }
    if let Some(h) = &home {
        dirs.push(h.join(".local/share/flatpak/exports/share"));
    }
    let system = std::env::var_os("XDG_DATA_DIRS")
        .and_then(|v| v.to_str().map(str::to_owned))
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_owned());
    dirs.extend(
        system
            .split(':')
            .filter(|d| !d.is_empty())
            .map(PathBuf::from),
    );
    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share"));
    dirs.push(PathBuf::from("/var/lib/snapd/desktop"));
    dirs
}

/// Look an icon name up in the hicolor theme and `pixmaps`. Only the
/// standard theme is searched: it is where applications install their own
/// icon, and it avoids depending on the desktop environment's theme
/// engine. Prefers large PNGs, then scalable SVG.
fn find_icon(name: &str, data_dirs: &[PathBuf]) -> Option<PathBuf> {
    const SIZES: &[&str] = &["256x256", "128x128", "96x96", "64x64", "48x48", "scalable"];
    let path = Path::new(name);
    if path.is_absolute() {
        return super::existing_absolute(path.to_path_buf());
    }
    for dir in data_dirs {
        for size in SIZES {
            let ext = if *size == "scalable" { "svg" } else { "png" };
            let candidate = dir
                .join("icons/hicolor")
                .join(size)
                .join("apps")
                .join(format!("{name}.{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        for ext in ["png", "svg"] {
            let candidate = dir.join("pixmaps").join(format!("{name}.{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Enumerate desktop entries and convert the HTTP handlers into browsers.
pub fn installed_browsers() -> Vec<Browser> {
    let dirs = data_dirs();
    let mut seen_ids = HashSet::new();
    let mut browsers = Vec::new();
    for dir in &dirs {
        let Ok(entries) = std::fs::read_dir(dir.join("applications")) else {
            continue;
        };
        for file in entries.flatten() {
            let path = file.path();
            if path.extension().is_none_or(|e| e != "desktop") {
                continue;
            }
            let Some(id) = path.file_name().map(std::ffi::OsStr::to_os_string) else {
                continue;
            };
            if !seen_ids.insert(id) {
                continue; // a higher-precedence directory already provided this id
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            if let Some(browser) = browser_from_entry(&text, &dirs) {
                browsers.push(browser);
            }
        }
    }
    browsers
}

fn browser_from_entry(text: &str, dirs: &[PathBuf]) -> Option<Browser> {
    let entry = parse_desktop_entry(text)?;
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    if !entry.shown_on(&desktop) || !entry.handles_http() {
        return None;
    }
    let (program, args) = split_exec(&entry.exec)?;
    let path = resolve_program(&program)?;
    let mut browser = Browser::new(entry.name, path);
    browser.args = args;
    browser.icon = entry.icon.as_deref().and_then(|i| find_icon(i, dirs));
    Some(browser)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIREFOX: &str = "[Desktop Entry]\nVersion=1.0\nName=Firefox Web Browser\nName[fr]=Navigateur Web Firefox\nComment=Browse the Web\nExec=firefox %u\nIcon=firefox\nType=Application\nMimeType=text/html;text/xml;x-scheme-handler/http;x-scheme-handler/https;\nActions=new-window;\n\n[Desktop Action new-window]\nName=Open a New Window\nExec=firefox --new-window %u\n";

    #[test]
    fn parses_main_group_only() {
        let e = parse_desktop_entry(FIREFOX).unwrap();
        assert_eq!(e.name, "Firefox Web Browser");
        assert_eq!(e.exec, "firefox %u");
        assert_eq!(e.icon.as_deref(), Some("firefox"));
        assert!(e.handles_http());
        assert!(!e.hidden);
    }

    #[test]
    fn ignores_non_applications_and_hidden() {
        assert!(parse_desktop_entry("[Desktop Entry]\nType=Link\nName=X\nExec=x\n").is_none());
        assert!(parse_desktop_entry("[Desktop Entry]\nType=Application\nName=X\n").is_none());
        let hidden = parse_desktop_entry(
            "[Desktop Entry]\nType=Application\nName=X\nExec=x\nNoDisplay=true\n",
        )
        .unwrap();
        assert!(hidden.hidden);
        let plain = parse_desktop_entry(
            "[Desktop Entry]\nType=Application\nName=Editor\nExec=ed %f\nMimeType=text/plain;\n",
        )
        .unwrap();
        assert!(!plain.handles_http());
    }

    #[test]
    fn only_show_in_and_not_show_in_are_honoured() {
        let stub = parse_desktop_entry("[Desktop Entry]\nType=Application\nName=Chromium\nExec=/usr/bin/false\nOnlyShowIn=UbuntuFrame;\nMimeType=x-scheme-handler/http;\n").unwrap();
        assert!(!stub.shown_on("KDE"));
        assert!(!stub.shown_on(""));
        assert!(stub.shown_on("UbuntuFrame"));
        assert!(stub.shown_on("GNOME:UbuntuFrame"));
        let excluded = parse_desktop_entry(
            "[Desktop Entry]\nType=Application\nName=X\nExec=x\nNotShowIn=KDE;\n",
        )
        .unwrap();
        assert!(!excluded.shown_on("KDE"));
        assert!(excluded.shown_on("GNOME"));
        let plain =
            parse_desktop_entry("[Desktop Entry]\nType=Application\nName=X\nExec=x\n").unwrap();
        assert!(plain.shown_on("KDE"));
        assert!(plain.shown_on(""));
    }

    #[test]
    fn split_exec_handles_field_codes_and_quotes() {
        assert_eq!(
            split_exec("firefox %u").unwrap(),
            ("firefox".into(), vec!["%URL%".into()])
        );
        assert_eq!(
            split_exec("/usr/bin/google-chrome-stable %U").unwrap(),
            ("/usr/bin/google-chrome-stable".into(), vec!["%URL%".into()])
        );
        assert_eq!(
            split_exec("/usr/bin/flatpak run --command=firefox --file-forwarding org.mozilla.firefox @@u %u @@").unwrap(),
            ("/usr/bin/flatpak".into(), vec!["run", "--command=firefox", "--file-forwarding", "org.mozilla.firefox", "@@u", "%URL%", "@@"].into_iter().map(String::from).collect())
        );
        assert_eq!(
            split_exec("env BAMF_DESKTOP_FILE_HINT=/x/y.desktop /snap/bin/firefox %u")
                .unwrap()
                .0,
            "env"
        );
        assert_eq!(
            split_exec(r#""/opt/My Browser/bin" --name="a b" %c %i %u"#).unwrap(),
            (
                "/opt/My Browser/bin".into(),
                vec!["--name=a b".into(), "%URL%".into()]
            )
        );
        assert_eq!(split_exec("x 100%%").unwrap().1, vec!["100%"]);
        assert!(split_exec("").is_none());
        assert!(split_exec("\"unterminated").is_none());
    }

    #[test]
    fn resolves_through_path_and_finds_icons() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        std::fs::create_dir(&bin).unwrap();
        std::fs::write(bin.join("mybrowser"), b"").unwrap();
        std::env::set_var("PATH", &bin);
        assert_eq!(resolve_program("mybrowser"), Some(bin.join("mybrowser")));
        assert!(resolve_program("nothing-here").is_none());
        assert_eq!(
            resolve_program(bin.join("mybrowser").to_str().unwrap()),
            Some(bin.join("mybrowser"))
        );

        let icons = dir.path().join("icons/hicolor/128x128/apps");
        std::fs::create_dir_all(&icons).unwrap();
        std::fs::write(icons.join("mybrowser.png"), b"").unwrap();
        assert_eq!(
            find_icon("mybrowser", &[dir.path().to_path_buf()]),
            Some(icons.join("mybrowser.png"))
        );
        assert!(find_icon("other", &[dir.path().to_path_buf()]).is_none());

        let entry = "[Desktop Entry]\nType=Application\nName=My Browser\nExec=mybrowser --x %u\nIcon=mybrowser\nMimeType=x-scheme-handler/https;\n";
        let b = browser_from_entry(entry, &[dir.path().to_path_buf()]).unwrap();
        assert_eq!(b.name, "My Browser");
        assert_eq!(b.path, bin.join("mybrowser"));
        assert_eq!(b.args, vec!["--x", "%URL%"]);
        assert_eq!(b.icon, Some(icons.join("mybrowser.png")));
    }
}
