//! Linux: a user-level desktop entry plus a direct edit of `mimeapps.list`
//! (see [`super::mimeapps`]). `xdg-settings` is also invoked when present,
//! for desktops that keep their own notion of the default browser, but its
//! failure is not fatal.

use std::path::{Path, PathBuf};

use super::{Outcome, run_tool};
use crate::PlatformError;

/// Program name Reroute gives GTK at start-up (see the app's `run`). On
/// Wayland it is the window's application id, and the desktop finds the
/// window's icon and name through the entry called `<id>.desktop`, case
/// included. The binary itself is `reroute`, which matches no entry.
pub const APP_ID: &str = "Reroute";

/// Desktop-entry id, named after [`APP_ID`]. The deb and rpm packages install
/// it system-wide (Tauri names the file after the product); an unpackaged
/// binary writes the same id at user level, so the icon is found either way.
pub const DESKTOP_ID: &str = "Reroute.desktop";

/// Id of the user-level entry that versions up to 0.1.10 wrote, before the
/// id followed [`APP_ID`]. Registering removes it so that the menu does not
/// list Reroute twice.
const LEGACY_DESKTOP_ID: &str = "reroute.desktop";

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

/// Is the packaged desktop entry installed in one of `data_dirs`? It is then
/// used as it is, and no second, user-level entry is written.
fn packaged_in(data_dirs: &[PathBuf]) -> bool {
    data_dirs
        .iter()
        .any(|d| d.join("applications").join(DESKTOP_ID).is_file())
}

/// Contents of the desktop entry pointing at `exe`.
pub fn desktop_file_contents(exe: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Reroute\nComment=Choose a browser for each link\nExec={} %u\nIcon=reroute\nTerminal=false\nCategories=Network;WebBrowser;\nMimeType=x-scheme-handler/http;x-scheme-handler/https;text/html;\nStartupWMClass={APP_ID}\nStartupNotify=false\nNoDisplay=false\n",
        quote_exec(exe)
    )
}

/// Quote a path for the `Exec=` key (double quotes; escape `"`, `` ` ``,
/// `$` and `\`).
pub(crate) fn quote_exec(path: &Path) -> String {
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

/// An XDG base directory: `$variable` when it holds an absolute path, as
/// the specification requires, else `fallback` under the home directory.
fn xdg_dir(variable: &str, fallback: &str) -> Result<PathBuf, PlatformError> {
    match absolute(std::env::var_os(variable)) {
        Some(dir) => Ok(dir),
        None => Ok(home()?.join(fallback)),
    }
}

fn absolute(value: Option<std::ffi::OsString>) -> Option<PathBuf> {
    value.map(PathBuf::from).filter(|dir| dir.is_absolute())
}

/// `$XDG_CONFIG_HOME`, usually `~/.config`.
pub(crate) fn user_config_dir() -> Result<PathBuf, PlatformError> {
    xdg_dir("XDG_CONFIG_HOME", ".config")
}

/// `$XDG_DATA_HOME`, usually `~/.local/share`.
fn user_data_dir() -> Result<PathBuf, PlatformError> {
    xdg_dir("XDG_DATA_HOME", ".local/share")
}

/// The program the desktop should start to run Reroute: the AppImage file
/// itself when Reroute runs from one (the executable inside sits in a mount
/// that disappears when Reroute exits), else this executable.
pub(crate) fn reroute_program() -> Result<PathBuf, PlatformError> {
    match appimage() {
        Some(image) => Ok(image),
        None => std::env::current_exe().map_err(|e| PlatformError::Os(e.to_string())),
    }
}

/// The AppImage file Reroute runs from, if it does.
fn appimage() -> Option<PathBuf> {
    absolute(std::env::var_os("APPIMAGE"))
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
    Ok(user_config_dir()?.join("mimeapps.list"))
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

/// Write (or refresh) `~/.local/share/applications/Reroute.desktop`.
pub fn ensure_desktop_file() -> Result<PathBuf, PlatformError> {
    let program = reroute_program()?;
    let dir = applications_dir()?;
    std::fs::create_dir_all(&dir).map_err(|e| PlatformError::Os(e.to_string()))?;
    let path = dir.join(DESKTOP_ID);
    std::fs::write(&path, desktop_file_contents(&program))
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
    remove_legacy_desktop_file()?;
    // An AppImage lists its own entry in `XDG_DATA_DIRS`, but that one goes
    // with the AppImage's mount: it needs a user-level entry all the same.
    if appimage().is_some() || !packaged_in(&system_data_dirs()) {
        let path = ensure_desktop_file()?;
        // Refresh only the directory we just wrote into. Called without an
        // argument the tool tries the system directories, which need root and
        // which the package manager already refreshed.
        if let Some(dir) = path.parent().and_then(std::path::Path::to_str)
            && let Err(error) = run_tool("update-desktop-database", &[dir])
        {
            log::info!("{error} (ignored)");
        }
    }
    // Some desktops keep their own notion of the default browser. On KDE this
    // one needs `qtpaths` and fails without it, which is why the authoritative
    // step below is our own edit of mimeapps.list.
    if let Err(error) = run_tool("xdg-settings", &["set", "default-web-browser", DESKTOP_ID]) {
        log::debug!("{error} (ignored)");
    }
    let path = mimeapps_path()?;
    let current = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(PlatformError::Os(format!("{}: {e}", path.display()))),
    };
    write_atomically(
        &path,
        &super::mimeapps::set_default_browser(&current, DESKTOP_ID),
    )?;
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
    if let Ok(text) = std::fs::read_to_string(&path)
        && let Some(handlers) = super::mimeapps::default_web_handlers(&text)
    {
        return Ok(handlers.iter().all(|h| h == DESKTOP_ID));
    }
    let current = run_tool("xdg-settings", &["get", "default-web-browser"])?;
    Ok(current.trim() == DESKTOP_ID)
}

/// Remove the user-level entry older versions wrote (see
/// [`LEGACY_DESKTOP_ID`]). Only Reroute ever wrote a file of that name.
fn remove_legacy_desktop_file() -> Result<(), PlatformError> {
    let path = applications_dir()?.join(LEGACY_DESKTOP_ID);
    match std::fs::remove_file(&path) {
        Ok(()) => {
            log::info!("removed the old desktop entry {}", path.display());
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(PlatformError::Os(format!("{}: {e}", path.display()))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_file_declares_http_handlers_and_quotes_exec() {
        let text = desktop_file_contents(Path::new("/opt/sign post/bin/reroute"));
        assert!(text.contains("Exec=\"/opt/sign post/bin/reroute\" %u"));
        assert!(text.contains("x-scheme-handler/https;"));
        assert!(text.contains("StartupWMClass=Reroute\n"));
        assert!(text.starts_with("[Desktop Entry]\n"));
    }

    #[test]
    fn the_desktop_entry_is_named_after_the_application_id() {
        // Wayland looks the icon up through `<app id>.desktop`, exactly.
        assert_eq!(DESKTOP_ID, format!("{APP_ID}.desktop"));
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
    fn the_packaged_entry_is_found_when_installed() {
        let dir = tempfile::tempdir().unwrap();
        let dirs = [dir.path().to_path_buf()];
        assert!(!packaged_in(&dirs));
        std::fs::create_dir_all(dir.path().join("applications")).unwrap();
        std::fs::write(
            dir.path().join("applications").join(DESKTOP_ID),
            "[Desktop Entry]\n",
        )
        .unwrap();
        assert!(packaged_in(&dirs));
    }

    #[test]
    fn xdg_directories_must_be_absolute() {
        assert_eq!(
            absolute(Some("/home/a/.config".into())),
            Some(PathBuf::from("/home/a/.config"))
        );
        assert_eq!(absolute(Some("relative/.config".into())), None);
        assert_eq!(absolute(None), None);
    }

    #[test]
    fn exec_quoting_escapes_specials() {
        assert_eq!(
            quote_exec(Path::new("/a/b\"c$d`e\\f")),
            "\"/a/b\\\"c\\$d\\`e\\\\f\""
        );
    }
}
