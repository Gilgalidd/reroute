//! The channel through which a new Reroute process hands its request to the
//! one running in the background (see [`reroute_core::control`]): a Unix
//! socket on Linux and macOS, a named pipe on Windows. Both offer the same three
//! calls: [`claim`] to become the Reroute that stays, [`Listener::serve`] to
//! answer, [`deliver`] to hand a request over.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use unix::{Listener, claim, deliver};
#[cfg(windows)]
pub use windows::{Listener, claim, deliver, session};
