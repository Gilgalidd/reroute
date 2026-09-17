//! Windows: register a `ProgId` and a `StartMenuInternet` client under
//! `HKEY_CURRENT_USER` (no administrator rights needed), then open the
//! Settings page where the user must pick Signpost. Since Windows 8 there
//! is no supported way for an application to set itself as default.

use std::path::Path;

use super::Outcome;
use crate::PlatformError;

/// `ProgId` associated with `http`/`https`.
pub const PROG_ID: &str = "SignpostURL";
/// Client name under `Software\Clients\StartMenuInternet`.
pub const CLIENT: &str = "Signpost";

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
        (prog_key.clone(), "", "Signpost URL".into()),
        (prog_key.clone(), "FriendlyTypeName", "Signpost URL".into()),
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
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

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
        "Windows only lets you choose the default browser yourself: in the Settings page that just opened, select Signpost.".into(),
    ))
}

/// `explorer.exe` understands `ms-settings:` URIs; using it avoids both a
/// shell and unsafe FFI.
#[cfg(windows)]
fn open_default_apps_settings() -> Result<(), PlatformError> {
    let system_root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    let explorer = Path::new(&system_root).join("explorer.exe");
    signpost_core::browser::check_executable(&explorer)
        .map_err(|e| PlatformError::Os(e.to_string()))?;
    std::process::Command::new(explorer)
        .arg(format!(
            "ms-settings:defaultapps?registeredAppUser={CLIENT}"
        ))
        .spawn()
        .map(drop)
        .map_err(|e| PlatformError::Tool {
            tool: "explorer.exe",
            reason: e.to_string(),
        })
}

/// Read the user's choice for `https` and `http`.
#[cfg(windows)]
pub fn is_default() -> Result<bool, PlatformError> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;

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
    fn entries_describe_a_complete_browser_registration() {
        let entries = registry_entries(Path::new(r"C:\Program Files\Signpost\signpost.exe"));
        let get = |k: &str, n: &str| {
            entries
                .iter()
                .find(|(sk, vn, _)| sk == k && *vn == n)
                .map(|(_, _, d)| d.clone())
        };
        assert_eq!(
            get(r"Software\Classes\SignpostURL\shell\open\command", "").unwrap(),
            r#""C:\Program Files\Signpost\signpost.exe" "%1""#
        );
        assert_eq!(
            get(
                r"Software\Clients\StartMenuInternet\Signpost\Capabilities\URLAssociations",
                "https"
            )
            .unwrap(),
            "SignpostURL"
        );
        assert_eq!(
            get(r"Software\RegisteredApplications", "Signpost").unwrap(),
            r"Software\Clients\StartMenuInternet\Signpost\Capabilities"
        );
        assert_eq!(entries.len(), 13);
    }
}
