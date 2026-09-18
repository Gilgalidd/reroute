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
//! * [`paths::config_dir`] — where `config.toml` lives.
//!
//! Each OS-specific module keeps a pure, unit-tested parsing layer
//! (desktop entries, registry command lines, `Info.plist`) separate from
//! the thin layer that enumerates the real system.

#![cfg_attr(not(target_os = "macos"), forbid(unsafe_code))]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod discover;
pub mod icons;
pub mod launch;
pub mod paths;
pub mod register;

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
