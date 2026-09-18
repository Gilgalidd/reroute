//! Well-known locations.

use std::path::PathBuf;

/// Environment variable that overrides the configuration directory. Handy
/// for tests, portable installs and reproducing bug reports.
pub const CONFIG_DIR_ENV: &str = "REROUTE_CONFIG_DIR";

/// Directory holding `config.toml`:
///
/// * Linux: `$XDG_CONFIG_HOME/reroute` (usually `~/.config/reroute`)
/// * macOS: `~/Library/Application Support/dev.reroute.app`
/// * Windows: `%APPDATA%\reroute\reroute\config`
///
/// `REROUTE_CONFIG_DIR` takes precedence when set to an absolute path.
pub fn config_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os(CONFIG_DIR_ENV) {
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
        // Environment is process-global; keep this the only test touching it.
        std::env::set_var(CONFIG_DIR_ENV, "relative/dir");
        let dir = config_dir().unwrap();
        assert!(dir.is_absolute());
        assert!(!dir.ends_with("relative/dir"));
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var(CONFIG_DIR_ENV, tmp.path());
        assert_eq!(config_dir().unwrap(), tmp.path());
        std::env::remove_var(CONFIG_DIR_ENV);
    }
}
