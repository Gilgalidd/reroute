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
    absolute_override(override_dir).or_else(|| project_dirs().map(|d| d.config_dir().to_path_buf()))
}

/// Directory for what Reroute can rebuild, such as the last version check:
/// `~/.cache/reroute`, `~/Library/Caches/dev.reroute.reroute` or
/// `%LOCALAPPDATA%\reroute\reroute\cache`. With `REROUTE_CONFIG_DIR` set,
/// the configuration directory itself, so that a separate configuration
/// keeps a separate cache.
pub fn cache_dir() -> Option<PathBuf> {
    absolute_override(std::env::var_os(CONFIG_DIR_ENV))
        .or_else(|| project_dirs().map(|d| d.cache_dir().to_path_buf()))
}

/// The socket through which a new Reroute reaches the one running in the
/// background, in `$XDG_RUNTIME_DIR/reroute/`: the runtime directory belongs
/// to the user alone. Its name follows the configuration directory, so that
/// a Reroute started with another `REROUTE_CONFIG_DIR` keeps to itself.
/// `None` without a runtime directory; Reroute then does not stay running.
#[cfg(unix)]
pub fn control_socket() -> Option<PathBuf> {
    let runtime = PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR")?);
    if !runtime.is_absolute() {
        return None;
    }
    let config = config_dir()?;
    Some(socket_in(&runtime, &config))
}

/// [`control_socket`] for the given runtime and configuration directories.
#[cfg(unix)]
fn socket_in(runtime: &std::path::Path, config: &std::path::Path) -> PathBuf {
    let name = format!(
        "control-{:08x}.sock",
        fnv1a(config.as_os_str().as_encoded_bytes())
    );
    runtime.join("reroute").join(name)
}

/// FNV-1a, 32 bits: a short name that stays the same from one build to the
/// next, which the standard library's hasher does not promise.
#[cfg(unix)]
fn fnv1a(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c_9dc5, |hash, byte| {
        (hash ^ u32::from(*byte)).wrapping_mul(0x0100_0193)
    })
}

fn absolute_override(value: Option<OsString>) -> Option<PathBuf> {
    let dir = PathBuf::from(value?);
    if dir.is_absolute() {
        Some(dir)
    } else {
        log::warn!("{CONFIG_DIR_ENV} is not absolute; ignoring");
        None
    }
}

fn project_dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from("dev", "reroute", "reroute")
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

    #[cfg(unix)]
    #[test]
    fn each_configuration_has_its_own_short_socket() {
        let runtime = std::path::Path::new("/run/user/1000");
        let a = socket_in(runtime, std::path::Path::new("/home/a/.config/reroute"));
        let b = socket_in(runtime, std::path::Path::new("/tmp/other"));
        assert_ne!(a, b);
        assert!(a.starts_with("/run/user/1000/reroute/"));
        assert_eq!(
            a,
            socket_in(runtime, std::path::Path::new("/home/a/.config/reroute"))
        );
        // Unix socket paths are limited to about 100 bytes.
        assert!(a.as_os_str().len() < 60, "{}", a.display());
        assert_eq!(fnv1a(b"a"), 0xe40c_292c, "FNV-1a reference value");
    }
}
