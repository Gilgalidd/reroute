//! Staying in the background (Linux).
//!
//! With `run_in_background` on, the first Reroute to start keeps running
//! when its windows close, with the picker loaded but hidden, and the
//! session starts it again at the next login. A later `reroute <url>` hands
//! its request over [`reroute_platform::control`] and exits within
//! milliseconds; the picker appears as soon as its page has read the link,
//! instead of after GTK and WebKit have started from scratch. Measured on a
//! KDE Wayland laptop: the click's process exits after 50–80 ms and the
//! picker shows after 105–165 ms, against about 0.8 s from cold.

use std::time::Duration;

use reroute_core::control::Request;
use reroute_platform::control::{self, Listener};
use tauri::{AppHandle, Manager};

use crate::cli::Mode;
use crate::incoming::{self, Decision};
use crate::state::AppState;
use crate::{updates, windows};

/// What this process should do once its configuration is loaded.
pub enum Role {
    /// Stay in the background and serve later processes.
    Resident(Listener),
    /// Handle this request and exit, as without background mode.
    Alone,
    /// Another Reroute took the request: exit now.
    Done,
}

/// The request this process was started for, as the running Reroute
/// receives it. A background start carries none.
fn request_for(mode: &Mode) -> Option<Request> {
    match mode {
        Mode::Pick(raw) => Some(Request::Open(raw.clone())),
        Mode::Settings => Some(Request::Settings),
        Mode::Background | Mode::Version | Mode::MakeDefault => None,
    }
}

/// First thing at start-up: give this process's request to a Reroute that
/// already runs in the background. True when it took it.
pub fn hand_over(mode: &Mode) -> bool {
    match (reroute_platform::paths::control_socket(), request_for(mode)) {
        (Some(socket), Some(request)) => control::deliver(&socket, &request),
        _ => false,
    }
}

/// Once the configuration is loaded: become the Reroute that stays in the
/// background, if the settings ask for it and no other one does.
pub fn claim(mode: &Mode, state: &AppState) -> Role {
    if !state.config().settings.run_in_background {
        return Role::Alone;
    }
    let Some(socket) = reroute_platform::paths::control_socket() else {
        return Role::Alone;
    };
    match control::claim(&socket) {
        Ok(Some(listener)) => Role::Resident(listener),
        Ok(None) => {
            // Another Reroute started a moment ago and may not listen yet.
            let Some(request) = request_for(mode) else {
                return Role::Done;
            };
            for _ in 0..10 {
                if control::deliver(&socket, &request) {
                    return Role::Done;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Role::Alone
        }
        Err(error) => {
            log::warn!("cannot stay in the background: {error}");
            Role::Alone
        }
    }
}

/// Answer later processes, each request on the interface thread.
pub fn serve(listener: Listener, app: &AppHandle) {
    let app = app.clone();
    listener.serve(move |request| {
        let handle = app.clone();
        if let Err(error) = app.run_on_main_thread(move || dispatch(&handle, request)) {
            log::error!("cannot handle a request from another Reroute: {error}");
        }
    });
}

/// Do what `reroute <url>` or `reroute` would have done in its own process.
fn dispatch(app: &AppHandle, request: Request) {
    let state = app.state::<AppState>();
    let outcome = match request {
        Request::Open(raw) => {
            state.reload_if_changed();
            match incoming::decide(&state, &raw) {
                Decision::Launched => Ok(()),
                Decision::Ask(url) => {
                    state.set_pending(url);
                    windows::request_picker(app)
                }
                Decision::Refused(message) => {
                    state.set_url_error(message);
                    windows::request_picker(app)
                }
            }
        }
        Request::Settings => windows::open_settings(app),
    };
    if let Err(error) = outcome {
        log::error!("cannot open a window: {error}");
    }
    updates::refresh_if_due(app);
}
