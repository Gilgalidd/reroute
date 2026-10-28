//! Windows: starting Reroute when the user signs in, so that it runs in the
//! background and the picker opens at once after a click. This is a value
//! in the user's `Run` key, which Windows starts at sign-in and which Task
//! Manager's *Startup apps* page lists and can turn off.

use std::path::Path;

use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};

use crate::PlatformError;

/// The key, under `HKEY_CURRENT_USER`, whose values Windows starts at
/// sign-in.
pub const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// Reroute's value in [`RUN_KEY`]. The NSIS uninstaller removes it too
/// (`apps/desktop/src-tauri/windows/hooks.nsh`).
pub const VALUE: &str = "Reroute";

/// The command line that starts `exe` in the background.
pub fn command_line(exe: &Path) -> String {
    format!("\"{}\" --background", exe.display())
}

/// Add the value (refreshing its program path) or remove it. Turning
/// Reroute off in Task Manager still counts: Windows keeps that choice
/// apart, under `StartupApproved`, which Reroute leaves alone.
pub fn set(enabled: bool) -> Result<(), PlatformError> {
    let os = |e: std::io::Error| PlatformError::Os(format!(r"HKCU\{RUN_KEY}\{VALUE}: {e}"));
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if enabled {
        let exe = std::env::current_exe().map_err(os)?;
        let (key, _) = hkcu.create_subkey(RUN_KEY).map_err(os)?;
        key.set_value(VALUE, &command_line(&exe)).map_err(os)
    } else {
        let removed = hkcu
            .open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)
            .and_then(|key| key.delete_value(VALUE));
        match removed {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(os(e)),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_value_starts_reroute_in_the_background() {
        assert_eq!(
            command_line(Path::new(r"C:\Users\a b\AppData\Local\Reroute\reroute.exe")),
            r#""C:\Users\a b\AppData\Local\Reroute\reroute.exe" --background"#
        );
    }
}
