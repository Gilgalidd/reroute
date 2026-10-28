//! Starting Reroute when the user signs in, so that it runs in the
//! background and the picker opens at once after a click: an XDG autostart
//! entry on Linux, a value in the registry's `Run` key on Windows, a launch
//! agent on macOS.

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(windows)]
pub mod windows;

#[cfg(target_os = "linux")]
pub use linux::set;
#[cfg(target_os = "macos")]
pub use macos::set;
#[cfg(windows)]
pub use windows::set;
