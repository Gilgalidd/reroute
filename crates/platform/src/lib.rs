//! Operating-system integration for Reroute.
//!
//! Everything here touches the host: the file system, the process table,
//! the registry, Launch Services. The API is deliberately tiny so that the
//! behaviour can be reviewed in one sitting:
//!
//! * [`discover::installed_browsers`] — list browsers the OS knows about.
//! * [`launch::launch`] — spawn a [`reroute_core::LaunchPlan`].
//! * [`register`] — make Reroute the default browser, or check if it is.
//! * [`icons::load_icon`] — read a local icon file for display.
//! * [`release::latest_release`] — the one network request: on demand, and
//!   at most once a day by itself.
//! * [`paths::config_dir`] — where `config.toml` lives.
//! * [`control`] (Unix) — the socket a new Reroute uses to reach the one
//!   running in the background, and [`autostart`] (Linux), which starts
//!   that one with the session.
//!
//! Each OS-specific module keeps a pure, unit-tested parsing layer
//! (desktop entries, registry command lines, `Info.plist`) separate from
//! the thin layer that enumerates the real system.

// `unsafe` only where an operating system offers no safe binding: Launch
// Services on macOS, `ShellExecuteW` on Windows (see docs/security.md).
#![cfg_attr(not(any(target_os = "macos", windows)), forbid(unsafe_code))]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

#[cfg(target_os = "linux")]
pub mod autostart;
#[cfg(unix)]
pub mod control;
pub mod discover;
pub mod icons;
pub mod launch;
pub mod paths;
pub mod register;
pub mod release;

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
