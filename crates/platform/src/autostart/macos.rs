//! macOS: starting Reroute when the user logs in, so that it runs in the
//! background and the picker opens at once after a click. This is a launch
//! agent, `~/Library/LaunchAgents/dev.reroute.desktop.plist`, which launchd
//! starts at login and which System Settings › General › Login Items lists
//! under *Allow in the Background*, where it can be turned off.

use std::path::{Path, PathBuf};

use crate::PlatformError;
use crate::register::macos::BUNDLE_ID;

/// The agent's file in `home`, named after its label.
pub fn agent_path(home: &Path) -> PathBuf {
    home.join("Library/LaunchAgents")
        .join(format!("{BUNDLE_ID}.plist"))
}

/// The launch agent that starts `exe` in the background at login. It does
/// not keep Reroute alive: a Reroute the user quits stays quit until the
/// next login.
pub fn agent(exe: &Path) -> plist::Value {
    let program = vec![
        plist::Value::from(exe.to_string_lossy().into_owned()),
        plist::Value::from("--background"),
    ];
    let mut agent = plist::Dictionary::new();
    agent.insert("Label".into(), BUNDLE_ID.into());
    agent.insert("ProgramArguments".into(), program.into());
    agent.insert("RunAtLoad".into(), true.into());
    // The picker must open at once: no background throttling.
    agent.insert("ProcessType".into(), "Interactive".into());
    agent.into()
}

/// Gatekeeper runs a downloaded app from a random read-only copy until it
/// is moved out of the download folder. That copy disappears; an agent must
/// not point at it.
pub fn is_translocated(exe: &Path) -> bool {
    exe.components()
        .any(|part| part.as_os_str() == "AppTranslocation")
}

/// Add the agent (refreshing its program path) or remove it.
pub fn set(enabled: bool) -> Result<(), PlatformError> {
    let home = directories::BaseDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .ok_or_else(|| PlatformError::Os("no home directory".into()))?;
    let path = agent_path(&home);
    let os = |e: std::io::Error| PlatformError::Os(format!("{}: {e}", path.display()));
    if enabled {
        let exe = std::env::current_exe().map_err(os)?;
        if is_translocated(&exe) {
            return Err(PlatformError::Os(
                "Reroute runs from a temporary copy; move it to Applications".into(),
            ));
        }
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(os)?;
        }
        agent(&exe)
            .to_file_xml(&path)
            .map_err(|e| PlatformError::Os(format!("{}: {e}", path.display())))
    } else {
        match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(os(e)),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_agent_starts_reroute_in_the_background_at_login() {
        let exe = "/Applications/Re & route.app/Contents/MacOS/reroute";
        let mut xml = Vec::new();
        agent(Path::new(exe)).to_writer_xml(&mut xml).unwrap();
        let value = plist::Value::from_reader_xml(xml.as_slice()).unwrap();
        let agent = value.as_dictionary().unwrap();
        assert_eq!(
            agent.get("Label").and_then(plist::Value::as_string),
            Some(BUNDLE_ID)
        );
        let program: Vec<&str> = agent
            .get("ProgramArguments")
            .and_then(plist::Value::as_array)
            .unwrap()
            .iter()
            .filter_map(plist::Value::as_string)
            .collect();
        assert_eq!(program, [exe, "--background"]);
        assert_eq!(
            agent.get("RunAtLoad").and_then(plist::Value::as_boolean),
            Some(true)
        );
        assert!(agent.get("KeepAlive").is_none(), "quitting must last");
    }

    #[test]
    fn the_agent_is_the_user_s_own() {
        assert_eq!(
            agent_path(Path::new("/Users/a")),
            Path::new("/Users/a/Library/LaunchAgents/dev.reroute.desktop.plist")
        );
    }

    #[test]
    fn a_translocated_copy_is_recognised() {
        assert!(is_translocated(Path::new(
            "/private/var/folders/x/T/AppTranslocation/1F2E/d/Reroute.app/Contents/MacOS/reroute"
        )));
        assert!(!is_translocated(Path::new(
            "/Applications/Reroute.app/Contents/MacOS/reroute"
        )));
    }
}
