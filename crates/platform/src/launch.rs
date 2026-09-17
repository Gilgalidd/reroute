//! Spawning a browser.
//!
//! The plan is executed with [`std::process::Command`], i.e. `execve` /
//! `CreateProcess` with an argument vector. There is no shell, no string
//! interpolation and no inherited standard streams. The child is detached
//! so that Signpost can exit immediately afterwards.

use std::process::{Command, Stdio};

use signpost_core::{LaunchError, LaunchPlan};

/// Start the program described by `plan` and return without waiting.
pub fn launch(plan: &LaunchPlan) -> Result<(), LaunchError> {
    plan.check_program()?;
    log::info!(
        "launching {} with {} argument(s)",
        plan.program.display(),
        plan.args.len()
    );

    let mut command = Command::new(&plan.program);
    command
        .args(&plan.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(dir) = plan.program.parent() {
        command.current_dir(dir);
    }
    detach(&mut command);

    command.spawn().map(drop).map_err(|e| LaunchError::Spawn {
        program: plan.program.display().to_string(),
        reason: e.to_string(),
    })
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
    fn refuses_missing_program_before_spawning() {
        let plan = LaunchPlan {
            program: PathBuf::from("/nonexistent/browser"),
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
