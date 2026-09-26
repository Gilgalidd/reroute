//! Spawning a browser.
//!
//! The plan is executed with [`std::process::Command`], i.e. `execve` /
//! `CreateProcess` with an argument vector. There is no shell, no string
//! interpolation and no inherited standard streams. The child is detached
//! so that Reroute can exit immediately afterwards, and it does not inherit
//! the variables Reroute sets for its own windows.

use std::process::{Command, Stdio};
use std::sync::{Mutex, PoisonError};

use reroute_core::{LaunchError, LaunchPlan};

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
pub fn launch(plan: &LaunchPlan) -> Result<(), LaunchError> {
    plan.check_program()?;
    log::info!(
        "launching {} with {} argument(s)",
        plan.program.display(),
        plan.args.len()
    );
    command_for(plan)
        .spawn()
        .map(drop)
        .map_err(|e| LaunchError::Spawn {
            program: plan.program.display().to_string(),
            reason: e.to_string(),
        })
}

/// The command that runs `plan`, ready to spawn.
fn command_for(plan: &LaunchPlan) -> Command {
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
    detach(&mut command);
    command
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
    use std::path::PathBuf;

    #[test]
    fn browsers_do_not_inherit_reroute_s_own_variables() {
        let plan = LaunchPlan {
            program: PathBuf::from("/usr/bin/x"),
            args: vec![],
        };
        keep_from_browsers("REROUTE_TEST_OWN_VARIABLE");
        let command = command_for(&plan);
        let removed = command
            .get_envs()
            .any(|(name, value)| name == "REROUTE_TEST_OWN_VARIABLE" && value.is_none());
        assert!(
            removed,
            "the variable is removed from the child's environment"
        );
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
        assert!(matches!(launch(&plan), Err(LaunchError::NotFound(_))));
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
        launch(&plan).unwrap();
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
