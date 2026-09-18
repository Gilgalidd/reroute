//! The browser catalogue and how a browser is started.
//!
//! A [`Browser`] is an executable plus a default argument list. It may have
//! several [`Launch`]es (profiles, private mode, kiosk…) that only differ by
//! their arguments. Turning a browser + URL into something runnable produces
//! a [`LaunchPlan`]: a program path and an argument *vector*. No shell is
//! ever involved, so quoting and injection are structurally impossible.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::LaunchError;
use crate::url::SafeUrl;

/// Token replaced by the URL when building the argument vector. It may be a
/// whole argument (`"%URL%"`) or part of one (`"--app=%URL%"`).
pub const URL_PLACEHOLDER: &str = "%URL%";

/// Stable identifier of a [`Browser`]. Rulesets refer to it, so it must
/// never change once written to the configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BrowserId(pub Uuid);

impl BrowserId {
    /// A fresh random id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for BrowserId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BrowserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Stable identifier of a [`Launch`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LaunchId(pub Uuid);

impl LaunchId {
    /// A fresh random id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for LaunchId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for LaunchId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// An alternative way to start a browser (a profile, a private window…).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Launch {
    /// Stable id, referenced by rulesets.
    #[serde(default)]
    pub id: LaunchId,
    /// Text shown in the context menu.
    pub name: String,
    /// Arguments passed to the executable. If no argument contains
    /// [`URL_PLACEHOLDER`], the URL is appended as the last argument.
    #[serde(default)]
    pub args: Vec<String>,
}

/// One entry of the picker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Browser {
    /// Stable id, referenced by rulesets.
    #[serde(default)]
    pub id: BrowserId,
    /// Display name.
    pub name: String,
    /// Absolute path to the executable. On macOS this is the binary inside
    /// the `.app` bundle, not the bundle directory.
    pub path: PathBuf,
    /// Default arguments. See [`Launch::args`] for the placeholder rule.
    #[serde(default)]
    pub args: Vec<String>,
    /// Hidden entries stay in the configuration (rules may still target
    /// them) but are not shown in the picker.
    #[serde(default)]
    pub hidden: bool,
    /// Optional path to a local PNG/SVG/ICO/ICNS file used as icon. Remote
    /// URLs are not supported by design (Reroute never uses the network).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<PathBuf>,
    /// Alternative launches shown in the entry's context menu.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub launches: Vec<Launch>,
}

impl Browser {
    /// Convenience constructor with a fresh id and no extras.
    pub fn new(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            id: BrowserId::new(),
            name: name.into(),
            path: path.into(),
            args: Vec::new(),
            hidden: false,
            icon: None,
            launches: Vec::new(),
        }
    }

    /// Find a launch by id.
    pub fn launch(&self, id: LaunchId) -> Option<&Launch> {
        self.launches.iter().find(|l| l.id == id)
    }

    /// Build the argument vector for `url`, using the default arguments or
    /// the given launch's arguments.
    pub fn plan(&self, url: &SafeUrl, launch: Option<LaunchId>) -> Result<LaunchPlan, LaunchError> {
        let args = match launch {
            Some(id) => {
                &self
                    .launch(id)
                    .ok_or_else(|| LaunchError::UnknownLaunch(id.to_string()))?
                    .args
            }
            None => &self.args,
        };
        Ok(LaunchPlan {
            program: self.path.clone(),
            args: substitute_url(args, url),
        })
    }
}

/// Exactly what will be spawned: a program and its argument vector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LaunchPlan {
    /// Absolute path of the executable.
    pub program: PathBuf,
    /// Arguments, URL already substituted.
    pub args: Vec<String>,
}

impl LaunchPlan {
    /// Verify that `program` is an absolute path to an existing, executable
    /// regular file. This is called right before spawning and is the last
    /// line of defence against a tampered configuration.
    pub fn check_program(&self) -> Result<(), LaunchError> {
        check_executable(&self.program)
    }
}

/// Replace [`URL_PLACEHOLDER`] in `args`, appending the URL if absent.
fn substitute_url(args: &[String], url: &SafeUrl) -> Vec<String> {
    let mut out: Vec<String> = args
        .iter()
        .map(|a| a.replace(URL_PLACEHOLDER, url.as_str()))
        .collect();
    if !args.iter().any(|a| a.contains(URL_PLACEHOLDER)) {
        out.push(url.as_str().to_owned());
    }
    out
}

/// See [`LaunchPlan::check_program`].
pub fn check_executable(path: &Path) -> Result<(), LaunchError> {
    let shown = path.display().to_string();
    if !path.is_absolute() {
        return Err(LaunchError::RelativePath(shown));
    }
    let meta = std::fs::metadata(path).map_err(|_| LaunchError::NotFound(shown.clone()))?;
    if !meta.is_file() {
        return Err(LaunchError::NotFound(shown));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o111 == 0 {
            return Err(LaunchError::NotExecutable(shown));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url() -> SafeUrl {
        SafeUrl::parse("https://example.com/a?b=c").unwrap()
    }

    #[test]
    fn appends_url_when_no_placeholder() {
        let b = Browser {
            args: vec!["--new-tab".into()],
            ..Browser::new("X", "/usr/bin/x")
        };
        let plan = b.plan(&url(), None).unwrap();
        assert_eq!(plan.program, PathBuf::from("/usr/bin/x"));
        assert_eq!(plan.args, vec!["--new-tab", "https://example.com/a?b=c"]);
    }

    #[test]
    fn substitutes_placeholder_inside_and_as_whole_argument() {
        let b = Browser {
            args: vec!["--app=%URL%".into(), "%URL%".into()],
            ..Browser::new("X", "/usr/bin/x")
        };
        let plan = b.plan(&url(), None).unwrap();
        assert_eq!(
            plan.args,
            vec![
                "--app=https://example.com/a?b=c",
                "https://example.com/a?b=c"
            ]
        );
    }

    #[test]
    fn uses_launch_args_when_launch_selected() {
        let launch = Launch {
            id: LaunchId::new(),
            name: "Private".into(),
            args: vec!["-p".into()],
        };
        let b = Browser {
            launches: vec![launch.clone()],
            ..Browser::new("X", "/usr/bin/x")
        };
        assert_eq!(
            b.plan(&url(), Some(launch.id)).unwrap().args,
            vec!["-p", "https://example.com/a?b=c"]
        );
        let unknown = LaunchId::new();
        assert_eq!(
            b.plan(&url(), Some(unknown)).unwrap_err(),
            LaunchError::UnknownLaunch(unknown.to_string())
        );
    }

    #[test]
    fn empty_args_yield_only_the_url() {
        let b = Browser::new("X", "/usr/bin/x");
        assert_eq!(
            b.plan(&url(), None).unwrap().args,
            vec!["https://example.com/a?b=c"]
        );
    }

    #[test]
    fn check_executable_rejects_relative_missing_and_dirs() {
        assert!(matches!(
            check_executable(Path::new("firefox")),
            Err(LaunchError::RelativePath(_))
        ));
        assert!(matches!(
            check_executable(Path::new("/definitely/not/here")),
            Err(LaunchError::NotFound(_))
        ));
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            check_executable(dir.path()),
            Err(LaunchError::NotFound(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn check_executable_needs_exec_bit() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("bin");
        std::fs::write(&file, b"#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(matches!(
            check_executable(&file),
            Err(LaunchError::NotExecutable(_))
        ));
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(check_executable(&file), Ok(()));
    }

    #[test]
    fn serde_round_trip_keeps_ids() {
        let b = Browser {
            launches: vec![Launch {
                id: LaunchId::new(),
                name: "P".into(),
                args: vec![],
            }],
            ..Browser::new("Firefox", "/usr/bin/firefox")
        };
        let text = toml::to_string(&b).unwrap();
        let back: Browser = toml::from_str(&text).unwrap();
        assert_eq!(back, b);
    }
}
