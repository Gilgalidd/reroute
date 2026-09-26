//! Linux: starting Reroute with the session, so that it runs in the
//! background and the picker opens at once after a click. This follows the
//! XDG Autostart specification: a desktop entry in
//! `$XDG_CONFIG_HOME/autostart/` that the session starts at login.

use std::path::{Path, PathBuf};

use crate::PlatformError;
use crate::register::linux::{home, quote_exec};

/// File name of the entry in the autostart directory.
pub const ENTRY: &str = "Reroute.desktop";

/// Contents of the entry that starts `exe` in the background.
pub fn entry_contents(exe: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Reroute\nComment=Keeps the browser picker ready so that it opens at once\nExec={} --background\nIcon=reroute\nTerminal=false\nNoDisplay=true\nX-GNOME-Autostart-enabled=true\n",
        quote_exec(exe)
    )
}

/// Add the entry (refreshing its program path) or remove it.
pub fn set(enabled: bool) -> Result<(), PlatformError> {
    let path = autostart_dir()?.join(ENTRY);
    let os = |e: std::io::Error| PlatformError::Os(format!("{}: {e}", path.display()));
    if enabled {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(os)?;
        }
        std::fs::write(&path, entry_contents(&program()?)).map_err(os)
    } else {
        match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(os(e)),
            _ => Ok(()),
        }
    }
}

/// The program to start: the `AppImage` file itself when Reroute runs from
/// one (its own executable sits in a mount that disappears), else this
/// executable.
fn program() -> Result<PathBuf, PlatformError> {
    match std::env::var_os("APPIMAGE").map(PathBuf::from) {
        Some(image) if image.is_absolute() => Ok(image),
        _ => std::env::current_exe().map_err(|e| PlatformError::Os(e.to_string())),
    }
}

fn autostart_dir() -> Result<PathBuf, PlatformError> {
    let config = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(dir) => PathBuf::from(dir),
        None => home()?.join(".config"),
    };
    Ok(config.join("autostart"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entry_starts_reroute_in_the_background() {
        let text = entry_contents(Path::new("/opt/re route/reroute"));
        assert!(text.starts_with("[Desktop Entry]\n"));
        assert!(text.contains("Exec=\"/opt/re route/reroute\" --background\n"));
        assert!(text.contains("NoDisplay=true\n"));
    }
}
