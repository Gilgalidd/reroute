//! Knowing whether a newer version is published, for the picker's update
//! button: automatically when Reroute starts, at most once a day and only
//! while `check_for_updates` is on, and whenever that button is pressed.
//!
//! The request itself is `reroute_platform::release::latest_release`: one
//! HTTPS GET of a fixed address that sends nothing about the user.

use std::time::{SystemTime, UNIX_EPOCH};

use reroute_core::release::{LastCheck, LatestRelease, UpdateStatus, check_due};
use reroute_platform::release::{latest_release, load_last_check, store_last_check};
use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;
use crate::windows::PICKER;

/// The running version.
const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// Event that tells the picker the status changed, with the new status.
pub const STATUS_EVENT: &str = "update-status";

/// At start-up: show what the last check found, before any new one.
pub fn restore(state: &AppState) {
    if !state.config().settings.check_for_updates {
        return;
    }
    let found = reroute_platform::paths::cache_dir()
        .and_then(|dir| load_last_check(&dir))
        .and_then(|last| last.version);
    if let Some(version) = found {
        state.set_update_status(UpdateStatus::compare(&version, CURRENT));
    }
}

/// Check in the background if the last check is a day old, and tell the
/// picker what it finds.
pub fn refresh_if_due(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !state.config().settings.check_for_updates {
        return;
    }
    let Some(dir) = reroute_platform::paths::cache_dir() else {
        return;
    };
    let now = unix_now();
    let last = load_last_check(&dir);
    if !check_due(last.as_ref(), now) || !state.begin_update_check() {
        return;
    }
    // Record the attempt before asking: a Reroute that exits before the
    // answer comes must not ask again at the next click.
    let attempt = LastCheck {
        checked_at: now,
        version: last.and_then(|last| last.version),
    };
    if let Err(error) = store_last_check(&dir, &attempt) {
        log::warn!("version check not recorded: {error}");
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let status = match check_now(&app) {
            Ok(release) => UpdateStatus::compare(&release.version, CURRENT),
            Err(reason) => UpdateStatus::Failed { reason },
        };
        let state = app.state::<AppState>();
        state.set_update_status(status.clone());
        state.end_update_check();
        if let Err(error) = app.emit_to(PICKER, STATUS_EVENT, status) {
            log::warn!("cannot tell the picker about the version check: {error}");
        }
    });
}

/// Ask now which version is newest, and remember the answer. Blocks for up
/// to the request's timeout, so it runs off the interface thread.
pub fn check_now(app: &AppHandle) -> Result<LatestRelease, String> {
    let state = app.state::<AppState>();
    match latest_release(CURRENT) {
        Ok(release) => {
            state.set_update_status(UpdateStatus::compare(&release.version, CURRENT));
            if let Some(dir) = reroute_platform::paths::cache_dir() {
                let check = LastCheck {
                    checked_at: unix_now(),
                    version: Some(release.version.clone()),
                };
                if let Err(error) = store_last_check(&dir, &check) {
                    log::warn!("version check not recorded: {error}");
                }
            }
            Ok(release)
        }
        Err(error) => {
            let reason = error.to_string();
            state.set_update_status(UpdateStatus::Failed {
                reason: reason.clone(),
            });
            Err(reason)
        }
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}
