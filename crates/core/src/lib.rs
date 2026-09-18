//! Platform-independent core of Reroute, a browser picker.
//!
//! This crate contains everything that can be reasoned about and tested
//! without touching the operating system:
//!
//! * [`SafeUrl`] — a URL that passed strict validation (scheme allow-list,
//!   size limit, no credentials, no control characters).
//! * [`Browser`] / [`Launch`] — the user's browser catalogue and how each
//!   one is started. [`LaunchPlan`] is the exact program + argument vector
//!   to spawn; there is never a shell in between.
//! * [`Pattern`] / [`Ruleset`] — the rule engine that can pick a browser
//!   without asking the user.
//! * [`Config`] — the TOML document persisted on disk, with referential
//!   integrity checks, and [`ConfigStore`] for atomic, private I/O.
//! * [`import`] — one-way converters from other tools (currently Hurl).
//! * [`Brand`] — recognises well-known browsers for fallback icons.
//!
//! The crate deliberately has no dependency on any GUI or OS API so that
//! its behaviour is fully covered by fast unit and property tests.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod brand;
pub mod browser;
pub mod config;
pub mod error;
pub mod import;
pub mod rules;
pub mod store;
pub mod url;

pub use brand::Brand;
pub use browser::{Browser, BrowserId, Launch, LaunchId, LaunchPlan, URL_PLACEHOLDER};
pub use config::{Config, MergeReport, Settings, Theme, CONFIG_VERSION};
pub use error::{ConfigError, Error, LaunchError, PatternError, UrlError};
pub use rules::{Match, Pattern, Ruleset};
pub use store::ConfigStore;
pub use url::SafeUrl;

#[cfg(test)]
pub(crate) mod testutil {
    /// Turn a Unix-style path into one that is absolute on every OS: Windows
    /// needs a drive letter for `Path::is_absolute` to hold.
    pub fn abs(path: &str) -> String {
        if cfg!(windows) {
            format!("C:{path}")
        } else {
            path.to_owned()
        }
    }
}
