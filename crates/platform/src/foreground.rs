//! Windows: whether the user is working with one of this program's windows.
//!
//! The picker closes when the user leaves it. On Windows, the focus events
//! of its window cannot tell: WebView2 moves the keyboard focus into the
//! page whenever a move or resize of the window begins, which the windowing
//! layer reports as a loss of focus, so that dragging the picker would close
//! it. The window in the foreground tells instead.

use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

/// Is the window in the foreground one of this process's? `None` when no
/// window is, as while Windows switches from one window to another.
#[allow(unsafe_code)]
pub fn is_this_process() -> Option<bool> {
    // SAFETY: takes nothing and returns a window handle, or null.
    let window = unsafe { GetForegroundWindow() };
    if window.is_null() {
        return None;
    }
    let mut process = 0;
    // SAFETY: `window` is a handle the system just returned; if the window
    // has closed since, the call fails and returns 0. The process id is
    // written to a local that outlives the call.
    let thread = unsafe { GetWindowThreadProcessId(window, &raw mut process) };
    (thread != 0).then_some(process == std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This test program has no window, so the one in the foreground, if
    /// any, is another program's.
    #[test]
    fn a_program_without_windows_is_never_in_front() {
        assert_ne!(is_this_process(), Some(true));
    }
}
