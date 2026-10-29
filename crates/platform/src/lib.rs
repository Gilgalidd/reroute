//! Operating-system integration for Reroute.
//!
//! Everything here touches the host: the file system, the process table,
//! the registry, Launch Services. The API is deliberately tiny so that the
//! behaviour can be reviewed in one sitting:
//!
//! * [`discover::installed_browsers`] — list browsers the OS knows about.
//! * [`launch::launch`] — spawn a [`reroute_core::LaunchPlan`].
//! * [`register`] — make Reroute the default browser, or check if it is.
//! * [`icons::load_icon`] — read a local icon file for display, or on
//!   Windows the icon inside a program.
//! * `foreground` (Windows) — whether one of Reroute's windows is in front,
//!   which tells when the user leaves the picker.
//! * [`release::latest_release`] — the one network request: on demand, and
//!   at most once a day by itself.
//! * [`paths::config_dir`] — where `config.toml` lives.
//! * [`control`] — the socket or named pipe a new Reroute uses to reach the
//!   one running in the background, and [`autostart`], which starts that one
//!   when the user signs in.
//!
//! Each OS-specific module keeps a pure, unit-tested parsing layer
//! (desktop entries, registry command lines, `Info.plist`) separate from
//! the thin layer that enumerates the real system.

// `unsafe` only where an operating system offers no safe binding: Launch
// Services on macOS; on Windows `ShellExecuteW`, the control pipe,
// `AllowSetForegroundWindow` and the foreground window (see
// docs/security.md).
#![cfg_attr(not(any(target_os = "macos", windows)), forbid(unsafe_code))]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub mod autostart;
#[cfg(any(unix, windows))]
pub mod control;
pub mod discover;
#[cfg(any(windows, test))]
mod exe_icon;
#[cfg(windows)]
pub mod foreground;
pub mod icons;
pub mod launch;
pub mod message;
pub mod paths;
pub mod register;
pub mod release;

use reroute_core::Message;

/// Errors from OS integration.
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    /// A helper program (`xdg-settings`, `open`…) was not found or failed.
    #[error("{tool} failed: {reason}")]
    Tool {
        /// Name of the helper.
        tool: &'static str,
        /// What went wrong.
        reason: String,
    },
    /// Registry / Launch Services / file error.
    #[error("{0}")]
    Os(String),
    /// Feature is not available on this operating system.
    #[error("not supported on this platform")]
    Unsupported,
    /// Icon file rejected.
    #[error("icon rejected: {0}")]
    Icon(String),
}

impl PlatformError {
    /// What to tell the user. The system's own words stay in English, as
    /// the `detail` of the message.
    pub fn message(&self) -> Message {
        let text = self.to_string();
        match self {
            Self::Tool { tool, reason } => {
                message::PLATFORM_TOOL.with([(*tool).into(), reason.as_str().into()], text)
            }
            Self::Os(detail) => message::PLATFORM_OS.with([detail.as_str().into()], text),
            Self::Unsupported => message::PLATFORM_UNSUPPORTED.with([], text),
            Self::Icon(detail) => message::PLATFORM_ICON.with([detail.as_str().into()], text),
        }
    }
}

impl From<PlatformError> for Message {
    fn from(error: PlatformError) -> Self {
        error.message()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tool_that_failed_is_named() {
        let error = PlatformError::Tool {
            tool: "xdg-settings",
            reason: "exit status 1".into(),
        };
        let message = error.message();
        assert_eq!(message.code, "platform.tool");
        assert_eq!(message.text, "xdg-settings failed: exit status 1");
        assert_eq!(
            serde_json::to_value(&message).unwrap()["params"],
            serde_json::json!({ "tool": "xdg-settings", "detail": "exit status 1" })
        );
    }
}
