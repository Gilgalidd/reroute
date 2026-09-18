//! Becoming the default browser.
//!
//! Each OS has its own ceremony; the common shape is: describe Signpost to
//! the system (desktop file, registry keys, bundle id), then ask the system
//! to make it the default. Windows never lets an application set itself as
//! default silently, so [`register`] there ends with the Settings page open
//! and returns [`Outcome::NeedsUserAction`].

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "linux")]
pub mod mimeapps;
#[cfg(windows)]
pub mod windows;

use crate::PlatformError;

/// What happened after [`register`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum Outcome {
    /// Signpost is now the default browser.
    Done,
    /// The system requires the user to confirm; the message says where.
    NeedsUserAction(String),
}

/// Is Signpost currently the default handler for `https`?
pub fn is_default() -> Result<bool, PlatformError> {
    #[cfg(target_os = "linux")]
    return linux::is_default();
    #[cfg(target_os = "macos")]
    return macos::is_default();
    #[cfg(windows)]
    return windows::is_default();
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    Err(PlatformError::Unsupported)
}

/// Ask the system to make Signpost the default browser.
pub fn register() -> Result<Outcome, PlatformError> {
    #[cfg(target_os = "linux")]
    return linux::register();
    #[cfg(target_os = "macos")]
    return macos::register();
    #[cfg(windows)]
    return windows::register();
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    Err(PlatformError::Unsupported)
}

/// Run a system helper with an argument vector and return its stdout.
/// The helper is looked up on `PATH` and must pass
/// [`signpost_core::browser::check_executable`].
#[cfg(target_os = "linux")]
pub(crate) fn run_tool(tool: &'static str, args: &[&str]) -> Result<String, PlatformError> {
    let program = find_on_path(tool).ok_or_else(|| PlatformError::Tool {
        tool,
        reason: "not found on PATH".into(),
    })?;
    signpost_core::browser::check_executable(&program).map_err(|e| PlatformError::Tool {
        tool,
        reason: e.to_string(),
    })?;
    let output = std::process::Command::new(&program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| PlatformError::Tool {
            tool,
            reason: e.to_string(),
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(PlatformError::Tool {
            tool,
            reason: format!("exit status {}: {stderr}", output.status),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(target_os = "linux")]
fn find_on_path(tool: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join(tool))
        .find(|p| p.is_file())
}
