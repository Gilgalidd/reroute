//! Spawning a browser.
//!
//! The plan is executed with [`std::process::Command`], i.e. `execve` /
//! `CreateProcess` with an argument vector. There is no shell, no string
//! interpolation and no inherited standard streams. The child is detached
//! so that Reroute can exit immediately afterwards.
//!
//! The browser gets the user's environment, not the one Reroute runs in:
//! not the variables Reroute sets for its own windows, not the ones an
//! AppImage sets for the GTK inside it, and as activation token only the
//! one of the click that asked for this browser.

#[cfg(target_os = "linux")]
use std::ffi::OsString;
#[cfg(target_os = "linux")]
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, PoisonError};

use reroute_core::{LaunchError, LaunchPlan};

/// Variable through which the desktop hands a program the right to bring
/// its window to the front (Wayland's `xdg-activation`).
pub const ACTIVATION_TOKEN: &str = "XDG_ACTIVATION_TOKEN";

/// Environment variables Reroute set for itself, which the browsers it
/// starts must not inherit (see [`keep_from_browsers`]).
static OWN_VARIABLES: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

/// Leave `name` out of the environment of every browser started from now
/// on. For a variable Reroute set for its own windows, such as a WebKit
/// rendering switch that a WebKit-based browser would otherwise pick up.
pub fn keep_from_browsers(name: &'static str) {
    OWN_VARIABLES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(name);
}

/// Start the program described by `plan` and return without waiting.
/// `activation` is the activation token of the click this browser answers,
/// if the desktop gave one.
pub fn launch(plan: &LaunchPlan, activation: Option<&str>) -> Result<(), LaunchError> {
    plan.check_program()?;
    log::info!(
        "launching {} with {} argument(s)",
        plan.program.display(),
        plan.args.len()
    );
    let child = command_for(plan, activation)
        .spawn()
        .map_err(|e| LaunchError::Spawn {
            program: plan.program.display().to_string(),
            reason: e.to_string(),
        })?;
    reap(child);
    Ok(())
}

/// The command that runs `plan`, ready to spawn.
fn command_for(plan: &LaunchPlan, activation: Option<&str>) -> Command {
    let mut command = Command::new(&plan.program);
    command
        .args(&plan.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(dir) = plan.program.parent() {
        command.current_dir(dir);
    }
    for name in OWN_VARIABLES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
    {
        command.env_remove(name);
    }
    // A token Reroute inherited is not this click's: in the background it
    // belongs to an earlier click, and the desktop has already used it.
    command.env_remove(ACTIVATION_TOKEN);
    if let Some(token) = activation {
        command.env(ACTIVATION_TOKEN, token);
    }
    #[cfg(target_os = "linux")]
    if std::env::var_os("APPIMAGE").is_some()
        && let Some(appdir) = std::env::var_os("APPDIR")
    {
        for (name, value) in appimage_changes(Path::new(&appdir), std::env::vars_os()) {
            match value {
                Some(value) => command.env(name, value),
                None => command.env_remove(name),
            };
        }
    }
    detach(&mut command);
    command
}

/// Variables that describe an AppImage rather than the user's session: the
/// AppImage runtime's own, and the GTK theme that Reroute's AppImage forces
/// to Adwaita for its windows.
#[cfg(target_os = "linux")]
const APPIMAGE_VARIABLES: &[&str] = &["APPDIR", "APPIMAGE", "ARGV0", "OWD", "GTK_THEME"];

/// How to give a browser back the user's environment when Reroute runs
/// from an AppImage mounted at `appdir`, or was started by an application
/// that does and passed its environment on. The AppImage's launcher points
/// GTK's modules, schemas and data directories inside the mount, which
/// disappears when the AppImage exits; a browser that inherited them could
/// fail to open a file dialog. Each change is a variable to set, or to remove
/// (`None`): a variable pointing inside the mount goes, and a list of paths
/// such as `XDG_DATA_DIRS` keeps its other entries.
#[cfg(target_os = "linux")]
fn appimage_changes(
    appdir: &Path,
    vars: impl IntoIterator<Item = (OsString, OsString)>,
) -> Vec<(OsString, Option<OsString>)> {
    // Guard against an `APPDIR` of `/` or a relative one, which would
    // match every path.
    if !appdir.is_absolute() || appdir.parent().is_none() {
        return Vec::new();
    }
    let mut changes = Vec::new();
    for (name, value) in vars {
        if name
            .to_str()
            .is_some_and(|n| APPIMAGE_VARIABLES.contains(&n))
        {
            changes.push((name, None));
            continue;
        }
        let Some(text) = value.to_str() else {
            continue;
        };
        let entries: Vec<&str> = text.split(':').collect();
        let kept: Vec<&str> = entries
            .iter()
            .copied()
            .filter(|entry| !Path::new(entry).starts_with(appdir))
            .collect();
        if kept.len() < entries.len() {
            let kept = kept.join(":");
            changes.push((name, (!kept.is_empty()).then(|| kept.into())));
        }
    }
    changes
}

/// Wait for the browser on a thread of its own. A Reroute that stays in the
/// background outlives most of the browsers it starts, as Firefox and
/// Chromium hand the link to their open window and exit; a child nobody
/// waits for would stay in the process table as a zombie until Reroute
/// exits.
fn reap(mut child: Child) {
    let spawned = std::thread::Builder::new()
        .name("reroute-reaper".into())
        .spawn(move || child.wait());
    if let Err(error) = spawned {
        log::warn!("cannot wait for the browser: {error}");
    }
}

#[cfg(unix)]
fn detach(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    // New process group: the browser must not receive our terminal signals.
    command.process_group(0);
}

#[cfg(windows)]
fn detach(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn plan() -> LaunchPlan {
        LaunchPlan {
            program: PathBuf::from("/usr/bin/x"),
            args: vec![],
        }
    }

    /// What a command does to one variable of the environment it inherits.
    #[derive(Debug, PartialEq)]
    enum Env {
        Untouched,
        Removed,
        Set(OsString),
    }

    fn env_of(command: &Command, name: &str) -> Env {
        match command.get_envs().find(|(n, _)| *n == name) {
            None => Env::Untouched,
            Some((_, None)) => Env::Removed,
            Some((_, Some(value))) => Env::Set(value.into()),
        }
    }

    #[test]
    fn browsers_do_not_inherit_reroute_s_own_variables() {
        keep_from_browsers("REROUTE_TEST_OWN_VARIABLE");
        let command = command_for(&plan(), None);
        assert_eq!(env_of(&command, "REROUTE_TEST_OWN_VARIABLE"), Env::Removed);
    }

    #[test]
    fn a_browser_gets_the_token_of_its_own_click_only() {
        let command = command_for(&plan(), Some("kwin-12"));
        assert_eq!(
            env_of(&command, ACTIVATION_TOKEN),
            Env::Set("kwin-12".into())
        );
        let command = command_for(&plan(), None);
        assert_eq!(
            env_of(&command, ACTIVATION_TOKEN),
            Env::Removed,
            "an inherited token is not passed on"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn an_appimage_s_environment_stays_with_reroute() {
        let vars = [
            (
                "XDG_DATA_DIRS",
                "/tmp/.mount_Re1/usr/share:/usr/share:/opt/share",
            ),
            ("GTK_PATH", "/tmp/.mount_Re1//usr/lib/gtk-3.0"),
            ("GTK_THEME", "Adwaita:light"),
            ("APPIMAGE", "/home/a/Reroute.AppImage"),
            ("HOME", "/home/a"),
            ("PATH", "/usr/bin:/bin"),
            ("NEAR_MISS", "/tmp/.mount_Re12/usr/lib"),
        ]
        .map(|(n, v)| (OsString::from(n), OsString::from(v)));
        let changes = appimage_changes(Path::new("/tmp/.mount_Re1"), vars.clone());
        let change = |name: &str| {
            changes
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.clone())
        };
        assert_eq!(
            change("XDG_DATA_DIRS"),
            Some(Some("/usr/share:/opt/share".into()))
        );
        assert_eq!(change("GTK_PATH"), Some(None));
        assert_eq!(change("GTK_THEME"), Some(None));
        assert_eq!(change("APPIMAGE"), Some(None));
        assert_eq!(change("HOME"), None);
        assert_eq!(change("PATH"), None);
        assert_eq!(change("NEAR_MISS"), None, "another mount is left alone");
        assert!(appimage_changes(Path::new("/"), vars.clone()).is_empty());
        assert!(appimage_changes(Path::new("mount"), vars).is_empty());
    }

    /// A browser that exits at once, as Firefox does when it hands the link
    /// to its open window, must not stay a zombie of the Reroute that
    /// started it.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_browser_that_exits_is_waited_for() {
        let child = Command::new("true").spawn().unwrap();
        let entry = PathBuf::from(format!("/proc/{}", child.id()));
        reap(child);
        for _ in 0..100 {
            if !entry.exists() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        panic!("the child that exited is still in the process table");
    }

    #[test]
    fn refuses_missing_program_before_spawning() {
        let plan = LaunchPlan {
            program: PathBuf::from(if cfg!(windows) {
                r"C:\nonexistent\browser"
            } else {
                "/nonexistent/browser"
            }),
            args: vec![],
        };
        assert!(matches!(launch(&plan, None), Err(LaunchError::NotFound(_))));
    }

    #[cfg(unix)]
    #[test]
    fn spawns_an_executable_with_arguments() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("marker");
        let script = dir.path().join("fake-browser");
        std::fs::write(
            &script,
            format!("#!/bin/sh\nprintf '%s' \"$1\" > '{}'\n", marker.display()),
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();

        let plan = LaunchPlan {
            program: script,
            args: vec!["https://example.com/".into()],
        };
        launch(&plan, None).unwrap();
        for _ in 0..50 {
            if marker.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_eq!(
            std::fs::read_to_string(&marker).unwrap(),
            "https://example.com/"
        );
    }
}
