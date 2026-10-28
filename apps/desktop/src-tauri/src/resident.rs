//! Staying in the background.
//!
//! With `run_in_background` on, the first Reroute to start keeps running
//! when its windows close, with the picker loaded but hidden, and the
//! system starts it again when the user next signs in. A later
//! `reroute <url>` hands its request over [`reroute_platform::control`] (a
//! Unix socket, a named pipe on Windows) and exits within milliseconds; the
//! picker appears as soon as its page has read the link, instead of after
//! the web view has started from scratch. Measured on a KDE Wayland laptop:
//! the click's process exits after 50–80 ms and the picker shows after
//! 105–165 ms, against about 0.8 s from cold. On Windows, WebView2 takes
//! about 1 s from cold.
//!
//! On macOS a click does not start a process: Launch Services hands the
//! link to the running Reroute as an event. The socket still serves
//! `reroute <url>` from a terminal, and keeps a second Reroute from staying
//! in the background beside the first.

use std::time::{Duration, Instant};

use reroute_core::control::Request;
use reroute_platform::control::{self, Listener};
use tauri::{AppHandle, Manager};

use crate::cli::Mode;
use crate::state::AppState;
use crate::{incoming, updates, windows};

/// How long a new Reroute keeps trying to reach one that has just started
/// and may not listen yet, before it handles its click itself.
const WAIT_FOR_A_STARTING_REROUTE: Duration = Duration::from_secs(1);

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
/// receives it, with the click's `activation` token. A background start
/// carries none. Neither does a start without arguments on macOS: Launch
/// Services starts Reroute that way for a click, then delivers the link as
/// an event that only this process can receive, so it must not hand over
/// and exit.
fn request_for(mode: &Mode, activation: Option<&str>) -> Option<Request> {
    match mode {
        Mode::Pick(raw) => Some(Request::Open {
            url: raw.clone(),
            activation: activation.map(str::to_owned),
        }),
        Mode::Settings => (!cfg!(target_os = "macos")).then_some(Request::Settings),
        Mode::Background | Mode::Version | Mode::MakeDefault => None,
    }
}

/// First thing at start-up: give this process's request to a Reroute that
/// already runs in the background. True when it took it.
pub fn hand_over(mode: &Mode, activation: Option<&str>) -> bool {
    match (
        reroute_platform::paths::control_socket(),
        request_for(mode, activation),
    ) {
        (Some(socket), Some(request)) => control::deliver(&socket, &request),
        _ => false,
    }
}

/// Once the configuration is loaded: become the Reroute that stays in the
/// background, if the settings ask for it and no other one does.
pub fn claim(mode: &Mode, activation: Option<&str>, state: &AppState) -> Role {
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
            // The attempts stop after a while, counted in time rather than
            // in tries: one that does not answer at all, a stopped process
            // for one, costs a timeout per try.
            let Some(request) = request_for(mode, activation) else {
                // A start at login finds one already running: nothing to
                // do. A start without arguments on macOS may bring a link
                // (see `request_for`): it is handled here.
                return if *mode == Mode::Background {
                    Role::Done
                } else {
                    Role::Alone
                };
            };
            let give_up_at = Instant::now() + WAIT_FOR_A_STARTING_REROUTE;
            while Instant::now() < give_up_at {
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
    // Edits made to config.toml by hand since the last request: the rules
    // must see them, and the settings window must not save over them.
    state.reload_if_changed();
    let outcome = match request {
        Request::Open { url, activation } => {
            if incoming::route(&state, &url, activation) {
                windows::request_picker(app)
            } else {
                Ok(())
            }
        }
        Request::Settings => windows::open_settings(app),
    };
    if let Err(error) = outcome {
        log::error!("cannot open a window: {error}");
    }
    updates::refresh_if_due(app);
}
