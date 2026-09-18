//! Windows: read the `StartMenuInternet` registry keys, which every browser
//! that wants to appear in "Default apps" must populate.

use std::path::PathBuf;

use reroute_core::Browser;

/// Split a registry `shell\open\command` value into program and arguments.
///
/// Follows the `CommandLineToArgvW` conventions for the cases that occur in
/// practice: an optionally quoted program path followed by whitespace-
/// separated arguments, where `"%1"` (or `%1`) is the URL placeholder.
pub fn parse_command(command: &str) -> Option<(PathBuf, Vec<String>)> {
    let command = command.trim();
    let (program, rest) = if let Some(stripped) = command.strip_prefix('"') {
        let (p, r) = stripped.split_once('"')?;
        (p, r)
    } else {
        match command.split_once(char::is_whitespace) {
            Some((p, r)) => (p, r),
            None => (command, ""),
        }
    };
    if program.is_empty() {
        return None;
    }
    let args = tokenize(rest)
        .into_iter()
        .map(|a| {
            if a == "%1" {
                reroute_core::URL_PLACEHOLDER.to_owned()
            } else {
                a
            }
        })
        .collect();
    Some((PathBuf::from(program), args))
}

fn tokenize(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut pending = false;
    for c in text.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                pending = true;
            }
            c if c.is_whitespace() && !in_quotes => {
                if pending {
                    words.push(std::mem::take(&mut current));
                    pending = false;
                }
            }
            c => {
                current.push(c);
                pending = true;
            }
        }
    }
    if pending {
        words.push(current);
    }
    words
}

/// Icon path from a `DefaultIcon` value (`"C:\x\y.ico",0`). Only `.ico`
/// and `.png` files are usable; icons embedded in executables are not
/// extracted (the UI falls back to a coloured tile).
pub fn icon_from_default_icon(value: &str) -> Option<PathBuf> {
    let path = value.trim().trim_matches('"');
    let path = path
        .rsplit_once(',')
        .map_or(path, |(p, _)| p)
        .trim()
        .trim_matches('"');
    let path = PathBuf::from(path);
    let is_image = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("ico") || e.eq_ignore_ascii_case("png"));
    is_image.then_some(path)
}

/// Enumerate `HKLM`/`HKCU\Software\Clients\StartMenuInternet`.
#[cfg(windows)]
pub fn installed_browsers() -> Vec<Browser> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::RegKey;

    const ROOTS: &[(&str, winreg::HKEY)] = &[
        (r"Software\Clients\StartMenuInternet", HKEY_LOCAL_MACHINE),
        (
            r"Software\WOW6432Node\Clients\StartMenuInternet",
            HKEY_LOCAL_MACHINE,
        ),
        (r"Software\Clients\StartMenuInternet", HKEY_CURRENT_USER),
    ];
    let mut browsers = Vec::new();
    for (path, hive) in ROOTS {
        let Ok(root) = RegKey::predef(*hive).open_subkey_with_flags(path, KEY_READ) else {
            continue;
        };
        for name in root.enum_keys().flatten() {
            let Ok(key) = root.open_subkey_with_flags(&name, KEY_READ) else {
                continue;
            };
            let display: String = key.get_value("").unwrap_or_else(|_| name.clone());
            let Ok(command_key) = key.open_subkey_with_flags(r"shell\open\command", KEY_READ)
            else {
                continue;
            };
            let Ok(command) = command_key.get_value::<String, _>("") else {
                continue;
            };
            let Some((program, args)) = parse_command(&command) else {
                continue;
            };
            let Some(program) = super::existing_absolute(program) else {
                continue;
            };
            let mut browser = Browser::new(display, program);
            browser.args = args;
            browser.icon = key
                .open_subkey_with_flags("DefaultIcon", KEY_READ)
                .and_then(|k| k.get_value::<String, _>(""))
                .ok()
                .and_then(|v| icon_from_default_icon(&v))
                .and_then(super::existing_absolute);
            browsers.push(browser);
        }
    }
    browsers
}

#[cfg(not(windows))]
#[allow(dead_code)]
pub fn installed_browsers() -> Vec<Browser> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_and_unquoted_commands() {
        let (p, a) =
            parse_command(r#""C:\Program Files\Mozilla Firefox\firefox.exe" -osint -url "%1""#)
                .unwrap();
        assert_eq!(
            p,
            PathBuf::from(r"C:\Program Files\Mozilla Firefox\firefox.exe")
        );
        assert_eq!(a, vec!["-osint", "-url", "%URL%"]);

        let (p, a) =
            parse_command(r#""C:\Program Files\Google\Chrome\Application\chrome.exe""#).unwrap();
        assert_eq!(
            p,
            PathBuf::from(r"C:\Program Files\Google\Chrome\Application\chrome.exe")
        );
        assert!(a.is_empty());

        let (p, a) = parse_command(r"C:\x\b.exe --single-argument %1").unwrap();
        assert_eq!(p, PathBuf::from(r"C:\x\b.exe"));
        assert_eq!(a, vec!["--single-argument", "%URL%"]);

        assert!(parse_command("").is_none());
        assert!(parse_command("\"\"").is_none());
    }

    #[test]
    fn tokenizer_keeps_quoted_groups_and_empty_quotes() {
        assert_eq!(tokenize(r#" a "b c" "" d"#), vec!["a", "b c", "", "d"]);
        assert!(tokenize("   ").is_empty());
    }

    #[test]
    fn default_icon_only_accepts_image_files() {
        assert_eq!(
            icon_from_default_icon(r#""C:\b\icon.ico",0"#),
            Some(PathBuf::from(r"C:\b\icon.ico"))
        );
        assert_eq!(
            icon_from_default_icon(r"C:\b\logo.PNG"),
            Some(PathBuf::from(r"C:\b\logo.PNG"))
        );
        assert!(icon_from_default_icon(r"C:\b\chrome.exe,0").is_none());
    }
}
