//! Well-known locations.

use std::ffi::OsString;
use std::path::PathBuf;

/// Environment variable that overrides the configuration directory. Handy
/// for tests, portable installs and reproducing bug reports.
pub const CONFIG_DIR_ENV: &str = "REROUTE_CONFIG_DIR";

/// Directory holding `config.toml`:
///
/// * Linux: `$XDG_CONFIG_HOME/reroute` (usually `~/.config/reroute`)
/// * macOS: `~/Library/Application Support/dev.reroute.reroute`
/// * Windows: `%APPDATA%\reroute\reroute\config`
///
/// `REROUTE_CONFIG_DIR` takes precedence when set to an absolute path.
pub fn config_dir() -> Option<PathBuf> {
    config_dir_with(std::env::var_os(CONFIG_DIR_ENV))
}

/// [`config_dir`] with the override passed in rather than read from the
/// environment, so that tests never have to change the process environment.
fn config_dir_with(override_dir: Option<OsString>) -> Option<PathBuf> {
    if let Some(dir) = override_dir {
        let dir = PathBuf::from(dir);
        if dir.is_absolute() {
            return Some(dir);
        }
        log::warn!("{CONFIG_DIR_ENV} is not absolute; ignoring");
    }
    directories::ProjectDirs::from("dev", "reroute", "reroute")
        .map(|d| d.config_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_must_be_absolute() {
        let dir = config_dir_with(Some("relative/dir".into())).unwrap();
        assert!(dir.is_absolute());
        assert!(!dir.ends_with("relative/dir"));
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(
            config_dir_with(Some(tmp.path().into())).unwrap(),
            tmp.path()
        );
        assert!(config_dir_with(None).unwrap().is_absolute());
    }
}
