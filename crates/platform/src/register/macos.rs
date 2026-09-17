//! macOS: Launch Services.
//!
//! This is the only place in Signpost that uses `unsafe`: two C functions
//! from the `CoreServices` framework. `LSSetDefaultHandlerForURLScheme` makes
//! macOS show its own "Do you want to change your default web browser?"
//! confirmation, so the change is never silent.

#![allow(unsafe_code)]

use core_foundation::base::TCFType;
use core_foundation::string::{CFString, CFStringRef};

use super::Outcome;
use crate::PlatformError;

/// Must match `identifier` in `apps/desktop/src-tauri/tauri.conf.json`.
pub const BUNDLE_ID: &str = "dev.signpost.app";

type OSStatus = i32;

#[link(name = "CoreServices", kind = "framework")]
extern "C" {
    fn LSSetDefaultHandlerForURLScheme(scheme: CFStringRef, bundle_id: CFStringRef) -> OSStatus;
    fn LSCopyDefaultHandlerForURLScheme(scheme: CFStringRef) -> CFStringRef;
}

/// Ask Launch Services to make this bundle the handler for `http` and `https`.
pub fn register() -> Result<Outcome, PlatformError> {
    let bundle = CFString::new(BUNDLE_ID);
    for scheme in ["http", "https"] {
        let scheme = CFString::new(scheme);
        // SAFETY: both arguments are valid, owned CFString objects that
        // outlive the call; the function only reads them.
        let status = unsafe {
            LSSetDefaultHandlerForURLScheme(
                scheme.as_concrete_TypeRef(),
                bundle.as_concrete_TypeRef(),
            )
        };
        if status != 0 {
            return Err(PlatformError::Os(format!(
                "LSSetDefaultHandlerForURLScheme failed with status {status}"
            )));
        }
    }
    Ok(Outcome::NeedsUserAction(
        "macOS asks you to confirm the change in the dialog it just showed.".into(),
    ))
}

/// Compare the current `https` handler with our bundle id.
pub fn is_default() -> Result<bool, PlatformError> {
    let scheme = CFString::new("https");
    // SAFETY: `scheme` is a valid CFString. The function returns a +1
    // reference (Copy rule) or NULL, which `wrap_under_create_rule` takes
    // ownership of after the null check.
    let current = unsafe {
        let raw = LSCopyDefaultHandlerForURLScheme(scheme.as_concrete_TypeRef());
        if raw.is_null() {
            return Ok(false);
        }
        CFString::wrap_under_create_rule(raw)
    };
    Ok(current.to_string().eq_ignore_ascii_case(BUNDLE_ID))
}
