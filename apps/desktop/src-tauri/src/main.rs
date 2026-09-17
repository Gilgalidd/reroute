//! Binary entry point; all logic lives in the `signpost_lib` library crate.

// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    signpost_lib::run();
}
