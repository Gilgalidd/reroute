//! Linux: a user-level desktop entry plus `xdg-settings` / `xdg-mime`.

use std::path::{Path, PathBuf};

use super::{run_tool, Outcome};
use crate::PlatformError;

/// Desktop-entry id registered with the system.
pub const DESKTOP_ID: &str = "signpost.desktop";

/// Contents of the desktop entry pointing at `exe`.
pub fn desktop_file_contents(exe: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Signpost\nComment=Choose a browser for each link\nExec={} %u\nIcon=signpost\nTerminal=false\nCategories=Network;WebBrowser;\nMimeType=x-scheme-handler/http;x-scheme-handler/https;text/html;\nStartupNotify=false\nNoDisplay=false\n",
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

fn applications_dir() -> Result<PathBuf, PlatformError> {
    let base = match std::env::var_os("XDG_DATA_HOME") {
        Some(d) => PathBuf::from(d),
        None => Path::new(
            &std::env::var_os("HOME").ok_or_else(|| PlatformError::Os("HOME is not set".into()))?,
        )
        .join(".local/share"),
    };
    Ok(base.join("applications"))
}

/// Write (or refresh) `~/.local/share/applications/signpost.desktop`.
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
pub fn register() -> Result<Outcome, PlatformError> {
    ensure_desktop_file()?;
    // Best effort: refreshes the desktop database when the tool exists.
    let _ = run_tool("update-desktop-database", &[]);
    run_tool("xdg-settings", &["set", "default-web-browser", DESKTOP_ID])?;
    run_tool(
        "xdg-mime",
        &[
            "default",
            DESKTOP_ID,
            "x-scheme-handler/http",
            "x-scheme-handler/https",
        ],
    )?;
    Ok(Outcome::Done)
}

/// Compare `xdg-settings get default-web-browser` with our id.
pub fn is_default() -> Result<bool, PlatformError> {
    let current = run_tool("xdg-settings", &["get", "default-web-browser"])?;
    Ok(current.trim() == DESKTOP_ID)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_file_declares_http_handlers_and_quotes_exec() {
        let text = desktop_file_contents(Path::new("/opt/sign post/bin/signpost"));
        assert!(text.contains("Exec=\"/opt/sign post/bin/signpost\" %u"));
        assert!(text.contains("x-scheme-handler/https;"));
        assert!(text.starts_with("[Desktop Entry]\n"));
    }

    #[test]
    fn exec_quoting_escapes_specials() {
        assert_eq!(
            quote_exec(Path::new("/a/b\"c$d`e\\f")),
            "\"/a/b\\\"c\\$d\\`e\\\\f\""
        );
    }
}
