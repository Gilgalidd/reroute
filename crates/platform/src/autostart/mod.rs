//! Starting Reroute when the user signs in, so that it runs in the
//! background and the picker opens at once after a click: an XDG autostart
//! entry on Linux, a value in the registry's `Run` key on Windows.

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(windows)]
pub mod windows;

#[cfg(target_os = "linux")]
pub use linux::set;
#[cfg(windows)]
pub use windows::set;
