//! Linux: a user-level desktop entry plus a direct edit of `mimeapps.list`
//! (see [`super::mimeapps`]). `xdg-settings` is also invoked when present,
//! for desktops that keep their own notion of the default browser, but its
//! failure is not fatal.

use std::path::{Path, PathBuf};

use super::{run_tool, Outcome};
use crate::PlatformError;

/// Desktop-entry id registered with the system.
pub const DESKTOP_ID: &str = "reroute.desktop";

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

fn applications_dir() -> Result<PathBuf, PlatformError> {
    let base = match std::env::var_os("XDG_DATA_HOME") {
        Some(d) => PathBuf::from(d),
        None => home()?.join(".local/share"),
    };
    Ok(base.join("applications"))
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
    let data_dir = match std::env::var_os("XDG_DATA_HOME") {
        Some(d) => PathBuf::from(d),
        None => home()?.join(".local/share"),
    };
    for (size, png) in icons {
        let path = icon_path(&data_dir, *size);
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
    ensure_desktop_file()?;
    for (tool, args) in [
        ("update-desktop-database", vec![]),
        (
            "xdg-settings",
            vec!["set", "default-web-browser", DESKTOP_ID],
        ),
    ] {
        if let Err(error) = run_tool(tool, &args) {
            log::info!("{error} (ignored)");
        }
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
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Some(handlers) = super::mimeapps::default_web_handlers(&text) {
            return Ok(handlers.iter().all(|h| h == DESKTOP_ID));
        }
    }
    let current = run_tool("xdg-settings", &["get", "default-web-browser"])?;
    Ok(current.trim() == DESKTOP_ID)
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
        std::env::set_var("XDG_DATA_HOME", dir.path());
        install_icons(&[(32, b"\x89PNG"), (64, b"\x89PNG")]).unwrap();
        std::env::remove_var("XDG_DATA_HOME");
        assert!(icon_path(dir.path(), 32).is_file());
        assert!(icon_path(dir.path(), 64).is_file());
    }

    #[test]
    fn exec_quoting_escapes_specials() {
        assert_eq!(
            quote_exec(Path::new("/a/b\"c$d`e\\f")),
            "\"/a/b\\\"c\\$d\\`e\\\\f\""
        );
    }
}
