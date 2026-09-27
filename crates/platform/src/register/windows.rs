//! Windows: register a `ProgId` and a `StartMenuInternet` client under
//! `HKEY_CURRENT_USER` (no administrator rights needed), then open the
//! Settings page where the user must pick Reroute. Since Windows 8 there
//! is no supported way for an application to set itself as default.

use std::path::Path;

use super::Outcome;
use crate::PlatformError;

/// `ProgId` associated with `http`/`https`.
pub const PROG_ID: &str = "RerouteURL";
/// Client name under `Software\Clients\StartMenuInternet`.
pub const CLIENT: &str = "Reroute";

/// One registry string value: (`subkey` under `HKCU`, value name, data).
/// An empty value name means the key's default value.
pub type Entry = (String, &'static str, String);

/// Every value written by [`register`], as pure data so that it can be
/// reviewed and unit-tested without a registry.
pub fn registry_entries(exe: &Path) -> Vec<Entry> {
    let exe = exe.display().to_string();
    let quoted = format!("\"{exe}\"");
    let client_key = format!(r"Software\Clients\StartMenuInternet\{CLIENT}");
    let prog_key = format!(r"Software\Classes\{PROG_ID}");
    vec![
        (prog_key.clone(), "", "Reroute URL".into()),
        (prog_key.clone(), "FriendlyTypeName", "Reroute URL".into()),
        (
            format!(r"{prog_key}\DefaultIcon"),
            "",
            format!("{quoted},0"),
        ),
        (
            format!(r"{prog_key}\shell\open\command"),
            "",
            format!("{quoted} \"%1\""),
        ),
        (client_key.clone(), "", CLIENT.into()),
        (
            format!(r"{client_key}\DefaultIcon"),
            "",
            format!("{quoted},0"),
        ),
        (format!(r"{client_key}\shell\open\command"), "", quoted),
        (
            format!(r"{client_key}\Capabilities"),
            "ApplicationName",
            CLIENT.into(),
        ),
        (
            format!(r"{client_key}\Capabilities"),
            "ApplicationDescription",
            "Choose a browser for each link".into(),
        ),
        (
            format!(r"{client_key}\Capabilities\StartMenu"),
            "StartMenuInternet",
            CLIENT.into(),
        ),
        (
            format!(r"{client_key}\Capabilities\URLAssociations"),
            "http",
            PROG_ID.into(),
        ),
        (
            format!(r"{client_key}\Capabilities\URLAssociations"),
            "https",
            PROG_ID.into(),
        ),
        (
            r"Software\RegisteredApplications".into(),
            CLIENT,
            format!(r"{client_key}\Capabilities"),
        ),
    ]
}

/// Write the entries and open Settings › Default apps.
#[cfg(windows)]
pub fn register() -> Result<Outcome, PlatformError> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let exe = std::env::current_exe().map_err(|e| PlatformError::Os(e.to_string()))?;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for (subkey, name, data) in registry_entries(&exe) {
        let (key, _) = hkcu
            .create_subkey(&subkey)
            .map_err(|e| PlatformError::Os(format!("{subkey}: {e}")))?;
        key.set_value(name, &data)
            .map_err(|e| PlatformError::Os(format!("{subkey}\\{name}: {e}")))?;
    }
    open_default_apps_settings()?;
    Ok(Outcome::NeedsUserAction(
        "Windows only lets you choose the default browser yourself: in the Settings page that just opened, select Reroute.".into(),
    ))
}

/// The Settings page that lists Reroute's default apps, from its entry in
/// `RegisteredApplications`. Windows 11 before the 2023-04 update opens the
/// Default apps page without selecting Reroute.
pub fn default_apps_uri() -> String {
    format!("ms-settings:defaultapps?registeredAppUser={CLIENT}")
}

/// Open [`default_apps_uri`]. Only the shell's `ShellExecuteW` understands
/// such an address: given to `explorer.exe`, an argument with a `?` passes
/// for a file path, and Explorer opens the Documents folder instead.
#[cfg(windows)]
#[allow(unsafe_code)]
fn open_default_apps_settings() -> Result<(), PlatformError> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let wide = |text: &str| -> Vec<u16> { text.encode_utf16().chain([0]).collect() };
    let operation = wide("open");
    let uri = wide(&default_apps_uri());
    // SAFETY: both strings are UTF-16, end with a NUL and live until the
    // call returns, which is all ShellExecuteW needs of them; it keeps no
    // pointer. The API allows a null window, parameters and directory.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            uri.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    // Success is any value above 32; the values up to 32 are error codes.
    if result.addr() > 32 {
        Ok(())
    } else {
        Err(PlatformError::Tool {
            tool: "ShellExecuteW",
            reason: format!("error {}", result.addr()),
        })
    }
}

/// Read the user's choice for `https` and `http`.
#[cfg(windows)]
pub fn is_default() -> Result<bool, PlatformError> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let choice = |scheme: &str| -> Option<String> {
        hkcu.open_subkey_with_flags(
            format!(
                r"Software\Microsoft\Windows\Shell\Associations\UrlAssociations\{scheme}\UserChoice"
            ),
            KEY_READ,
        )
        .ok()?
        .get_value("ProgId")
        .ok()
    };
    Ok(choice("https").as_deref() == Some(PROG_ID) && choice("http").as_deref() == Some(PROG_ID))
}

#[cfg(not(windows))]
#[allow(dead_code)]
pub fn register() -> Result<Outcome, PlatformError> {
    Err(PlatformError::Unsupported)
}

#[cfg(not(windows))]
#[allow(dead_code)]
pub fn is_default() -> Result<bool, PlatformError> {
    Err(PlatformError::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_page_names_the_registered_application() {
        let entries = registry_entries(Path::new(r"C:\Program Files\Reroute\reroute.exe"));
        let registered = entries
            .iter()
            .find(|(key, _, _)| key == r"Software\RegisteredApplications")
            .map(|(_, name, _)| *name)
            .unwrap();
        assert_eq!(
            default_apps_uri(),
            format!("ms-settings:defaultapps?registeredAppUser={registered}")
        );
    }

    #[test]
    fn entries_describe_a_complete_browser_registration() {
        let entries = registry_entries(Path::new(r"C:\Program Files\Reroute\reroute.exe"));
        let get = |k: &str, n: &str| {
            entries
                .iter()
                .find(|(sk, vn, _)| sk == k && *vn == n)
                .map(|(_, _, d)| d.clone())
        };
        assert_eq!(
            get(r"Software\Classes\RerouteURL\shell\open\command", "").unwrap(),
            r#""C:\Program Files\Reroute\reroute.exe" "%1""#
        );
        assert_eq!(
            get(
                r"Software\Clients\StartMenuInternet\Reroute\Capabilities\URLAssociations",
                "https"
            )
            .unwrap(),
            "RerouteURL"
        );
        assert_eq!(
            get(r"Software\RegisteredApplications", "Reroute").unwrap(),
            r"Software\Clients\StartMenuInternet\Reroute\Capabilities"
        );
        assert_eq!(entries.len(), 13);
    }
}
