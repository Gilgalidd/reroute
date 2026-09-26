//! Linux: a user-level desktop entry plus a direct edit of `mimeapps.list`
//! (see [`super::mimeapps`]). `xdg-settings` is also invoked when present,
//! for desktops that keep their own notion of the default browser, but its
//! failure is not fatal.

use std::path::{Path, PathBuf};

use super::{Outcome, run_tool};
use crate::PlatformError;

/// Desktop-entry id written for unpackaged binaries.
pub const DESKTOP_ID: &str = "reroute.desktop";
/// Desktop-entry id installed by the deb/rpm packages (Tauri names it after
/// the product). When present system-wide it is reused instead of writing a
/// second, user-level entry.
pub const PACKAGED_DESKTOP_ID: &str = "Reroute.desktop";

/// System data directories that may hold the packaged desktop entry.
fn system_data_dirs() -> Vec<PathBuf> {
    std::env::var_os("XDG_DATA_DIRS")
        .and_then(|v| v.to_str().map(str::to_owned))
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_owned())
        .split(':')
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .collect()
}

/// The desktop id to register: the packaged one if installed, else ours.
pub fn desktop_id() -> &'static str {
    desktop_id_in(&system_data_dirs())
}

/// [`desktop_id`] for the given system data directories.
fn desktop_id_in(data_dirs: &[PathBuf]) -> &'static str {
    let packaged = data_dirs
        .iter()
        .any(|d| d.join("applications").join(PACKAGED_DESKTOP_ID).is_file());
    if packaged {
        PACKAGED_DESKTOP_ID
    } else {
        DESKTOP_ID
    }
}

/// Contents of the desktop entry pointing at `exe`.
pub fn desktop_file_contents(exe: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Reroute\nComment=Choose a browser for each link\nExec={} %u\nIcon=reroute\nTerminal=false\nCategories=Network;WebBrowser;\nMimeType=x-scheme-handler/http;x-scheme-handler/https;text/html;\nStartupNotify=false\nNoDisplay=false\n",
        quote_exec(exe)
    )
}

/// Quote a path for the `Exec=` key (double quotes; escape `"`, `` ` ``,
/// `$` and `\`).
fn quote_exec(path: &Path) -> String {
    let raw = path.to_string_lossy();
    let mut out = String::with_capacity(raw.len() + 2);
    out.push('"');
    for c in raw.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

fn home() -> Result<PathBuf, PlatformError> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| PlatformError::Os("HOME is not set".into()))
}

/// `$XDG_DATA_HOME`, usually `~/.local/share`.
fn user_data_dir() -> Result<PathBuf, PlatformError> {
    match std::env::var_os("XDG_DATA_HOME") {
        Some(d) => Ok(PathBuf::from(d)),
        None => Ok(home()?.join(".local/share")),
    }
}

fn applications_dir() -> Result<PathBuf, PlatformError> {
    Ok(user_data_dir()?.join("applications"))
}

/// Path of the `size`×`size` application icon inside a hicolor theme rooted
/// at `data_dir` (`$XDG_DATA_HOME` or `/usr/share`). Packaged installs use
/// the same layout, so `Icon=reroute` in the desktop entry resolves either way.
pub fn icon_path(data_dir: &Path, size: u32) -> PathBuf {
    data_dir
        .join("icons/hicolor")
        .join(format!("{size}x{size}"))
        .join("apps")
        .join("reroute.png")
}

/// Install the application icons into the user's hicolor theme so that the
/// desktop entry (and therefore the window title bar and task bar under
/// Wayland) shows a logo when Reroute runs from outside a package.
pub fn install_icons(icons: &[(u32, &[u8])]) -> Result<(), PlatformError> {
    install_icons_into(&user_data_dir()?, icons)
}

/// [`install_icons`] into the hicolor theme under `data_dir`.
fn install_icons_into(data_dir: &Path, icons: &[(u32, &[u8])]) -> Result<(), PlatformError> {
    for (size, png) in icons {
        let path = icon_path(data_dir, *size);
        let os = |e: std::io::Error| PlatformError::Os(format!("{}: {e}", path.display()));
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(os)?;
        }
        std::fs::write(&path, png).map_err(os)?;
    }
    Ok(())
}

/// `$XDG_CONFIG_HOME/mimeapps.list`, the user's default-application registry.
fn mimeapps_path() -> Result<PathBuf, PlatformError> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(d) => PathBuf::from(d),
        None => home()?.join(".config"),
    };
    Ok(base.join("mimeapps.list"))
}

fn write_atomically(path: &Path, contents: &str) -> Result<(), PlatformError> {
    let os = |e: std::io::Error| PlatformError::Os(format!("{}: {e}", path.display()));
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(os)?;
    }
    let tmp = path.with_extension("list.tmp");
    std::fs::write(&tmp, contents).map_err(os)?;
    std::fs::rename(&tmp, path).map_err(os)
}

/// Write (or refresh) `~/.local/share/applications/reroute.desktop`.
pub fn ensure_desktop_file() -> Result<PathBuf, PlatformError> {
    let exe = std::env::current_exe().map_err(|e| PlatformError::Os(e.to_string()))?;
    let dir = applications_dir()?;
    std::fs::create_dir_all(&dir).map_err(|e| PlatformError::Os(e.to_string()))?;
    let path = dir.join(DESKTOP_ID);
    std::fs::write(&path, desktop_file_contents(&exe))
        .map_err(|e| PlatformError::Os(e.to_string()))?;
    Ok(path)
}

/// Register the desktop entry and make it the default browser.
///
/// Order matters: the desktop-specific helpers run first and may fail (on
/// KDE without `qtpaths`, `xdg-settings` even restores the previous browser
/// when its own write fails), then `mimeapps.list` is written last so that
/// it is authoritative. The result is verified before reporting success.
pub fn register() -> Result<Outcome, PlatformError> {
    let id = desktop_id();
    if id == DESKTOP_ID {
        let path = ensure_desktop_file()?;
        // Refresh only the directory we just wrote into. Called without an
        // argument the tool tries the system directories, which need root and
        // which the package manager already refreshed.
        if let Some(dir) = path.parent().and_then(std::path::Path::to_str) {
            if let Err(error) = run_tool("update-desktop-database", &[dir]) {
                log::info!("{error} (ignored)");
            }
        }
    }
    // Some desktops keep their own notion of the default browser. On KDE this
    // one needs `qtpaths` and fails without it, which is why the authoritative
    // step below is our own edit of mimeapps.list.
    if let Err(error) = run_tool("xdg-settings", &["set", "default-web-browser", id]) {
        log::debug!("{error} (ignored)");
    }
    let path = mimeapps_path()?;
    let current = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(PlatformError::Os(format!("{}: {e}", path.display()))),
    };
    write_atomically(&path, &super::mimeapps::set_default_browser(&current, id))?;
    if is_default()? {
        Ok(Outcome::Done)
    } else {
        Err(PlatformError::Os(format!(
            "{} was written but does not name Reroute",
            path.display()
        )))
    }
}

/// Every web type in `mimeapps.list` must point at Reroute; fall back to
/// `xdg-settings` when the file has no opinion at all.
pub fn is_default() -> Result<bool, PlatformError> {
    let path = mimeapps_path()?;
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Some(handlers) = super::mimeapps::default_web_handlers(&text) {
            let id = desktop_id();
            return Ok(handlers.iter().all(|h| h == id));
        }
    }
    let current = run_tool("xdg-settings", &["get", "default-web-browser"])?;
    Ok(current.trim() == desktop_id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_file_declares_http_handlers_and_quotes_exec() {
        let text = desktop_file_contents(Path::new("/opt/sign post/bin/reroute"));
        assert!(text.contains("Exec=\"/opt/sign post/bin/reroute\" %u"));
        assert!(text.contains("x-scheme-handler/https;"));
        assert!(text.starts_with("[Desktop Entry]\n"));
    }

    #[test]
    fn icons_follow_the_hicolor_layout() {
        assert_eq!(
            icon_path(Path::new("/usr/share"), 128),
            PathBuf::from("/usr/share/icons/hicolor/128x128/apps/reroute.png")
        );
        let dir = tempfile::tempdir().unwrap();
        install_icons_into(dir.path(), &[(32, b"\x89PNG"), (64, b"\x89PNG")]).unwrap();
        assert!(icon_path(dir.path(), 32).is_file());
        assert!(icon_path(dir.path(), 64).is_file());
    }

    #[test]
    fn packaged_entry_is_preferred_when_installed() {
        let dir = tempfile::tempdir().unwrap();
        let dirs = [dir.path().to_path_buf()];
        assert_eq!(desktop_id_in(&dirs), DESKTOP_ID);
        std::fs::create_dir_all(dir.path().join("applications")).unwrap();
        std::fs::write(
            dir.path().join("applications").join(PACKAGED_DESKTOP_ID),
            "[Desktop Entry]\n",
        )
        .unwrap();
        assert_eq!(desktop_id_in(&dirs), PACKAGED_DESKTOP_ID);
    }

    #[test]
    fn exec_quoting_escapes_specials() {
        assert_eq!(
            quote_exec(Path::new("/a/b\"c$d`e\\f")),
            "\"/a/b\\\"c\\$d\\`e\\\\f\""
        );
    }
}
