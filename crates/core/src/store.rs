//! Reading and writing `config.toml` safely.
//!
//! * Writes are atomic: the document is written to a sibling temporary file
//!   and renamed over the target, so a crash never leaves a truncated file.
//! * On Unix the file is created with mode `0600` and its directory with
//!   `0700`; the configuration lists executables to run, so no other user
//!   may edit it.
//! * A missing file is not an error: it yields [`Config::default`] so the
//!   first run works without any setup.

use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::error::Error;

/// File name inside the configuration directory.
pub const FILE_NAME: &str = "config.toml";

/// Handle to the on-disk configuration file.
#[derive(Debug, Clone)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    /// A store for the file at `path` (parent directories are created on
    /// first write).
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// A store for `dir/config.toml`.
    pub fn in_dir(dir: &Path) -> Self {
        Self::new(dir.join(FILE_NAME))
    }

    /// Where the file lives.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read and validate the file; a missing file yields the default.
    pub fn load(&self) -> Result<Config, Error> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => Ok(Config::from_toml(&text)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(source) => Err(self.io(source)),
        }
    }

    /// Validate and atomically write `config`.
    pub fn save(&self, config: &Config) -> Result<(), Error> {
        config.validate()?;
        let text = config.to_toml()?;
        if let Some(dir) = self.path.parent() {
            create_private_dir(dir).map_err(|e| self.io(e))?;
        }
        let tmp = self.path.with_extension("toml.tmp");
        write_private_file(&tmp, text.as_bytes()).map_err(|e| self.io(e))?;
        std::fs::rename(&tmp, &self.path).map_err(|e| self.io(e))
    }

    fn io(&self, source: std::io::Error) -> Error {
        Error::Io {
            path: self.path.display().to_string(),
            source,
        }
    }
}

fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(dir)
    }
}

fn write_private_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::Browser;
    use crate::testutil::abs;

    #[test]
    fn missing_file_yields_default() {
        let dir = tempfile::tempdir().unwrap();
        let store = ConfigStore::in_dir(&dir.path().join("nested"));
        assert_eq!(store.load().unwrap(), Config::default());
    }

    #[test]
    fn save_then_load_round_trips_and_creates_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let store = ConfigStore::in_dir(&dir.path().join("a").join("b"));
        let mut config = Config::default();
        config
            .browsers
            .push(Browser::new("Firefox", abs("/usr/bin/firefox")));
        store.save(&config).unwrap();
        assert_eq!(store.load().unwrap(), config);
        assert!(
            !store.path().with_extension("toml.tmp").exists(),
            "temp file must be gone"
        );
    }

    #[test]
    fn invalid_config_is_not_written() {
        let dir = tempfile::tempdir().unwrap();
        let store = ConfigStore::in_dir(dir.path());
        let mut config = Config::default();
        config.browsers.push(Browser::new("Bad", "relative/path"));
        assert!(store.save(&config).is_err());
        assert!(!store.path().exists());
    }

    #[test]
    fn corrupt_file_is_an_error_not_a_reset() {
        let dir = tempfile::tempdir().unwrap();
        let store = ConfigStore::in_dir(dir.path());
        std::fs::write(store.path(), "this is = not [valid").unwrap();
        assert!(matches!(store.load(), Err(Error::Config(_))));
    }

    #[cfg(unix)]
    #[test]
    fn file_and_dir_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let store = ConfigStore::in_dir(&dir.path().join("cfg"));
        store.save(&Config::default()).unwrap();
        let file_mode = std::fs::metadata(store.path())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        let dir_mode = std::fs::metadata(dir.path().join("cfg"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(file_mode, 0o600);
        assert_eq!(dir_mode, 0o700);
    }
}
