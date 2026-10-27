//! Linux: starting Reroute with the session, so that it runs in the
//! background and the picker opens at once after a click. This follows the
//! XDG Autostart specification: a desktop entry in
//! `$XDG_CONFIG_HOME/autostart/` that the session starts at login.

use std::path::Path;

use crate::PlatformError;
use crate::register::linux::{quote_exec, reroute_program, user_config_dir};

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
    let path = user_config_dir()?.join("autostart").join(ENTRY);
    let os = |e: std::io::Error| PlatformError::Os(format!("{}: {e}", path.display()));
    if enabled {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(os)?;
        }
        std::fs::write(&path, entry_contents(&reroute_program()?)).map_err(os)
    } else {
        match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(os(e)),
            _ => Ok(()),
        }
    }
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
