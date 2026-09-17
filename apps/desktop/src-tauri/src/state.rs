//! Process-wide state shared between the command handlers.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

use signpost_core::{Config, ConfigStore, SafeUrl};

/// Everything the windows need to know.
pub struct AppState {
    /// Where the configuration is read from and written to.
    pub store: ConfigStore,
    config: Mutex<Config>,
    pending: Mutex<Option<SafeUrl>>,
    url_error: Mutex<Option<String>>,
    config_error: Mutex<Option<String>>,
}

/// Lock a mutex, recovering from poisoning: our guarded data is always left
/// consistent because updates replace whole values.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl AppState {
    /// Load the configuration from disk. Problems are recorded rather than
    /// fatal, so a broken file never prevents the user from opening a link.
    /// On the very first run the installed browsers are discovered and saved.
    pub fn load() -> Self {
        let dir = signpost_platform::paths::config_dir().unwrap_or_else(|| {
            log::warn!("no configuration directory available; using the temporary directory");
            std::env::temp_dir().join("signpost")
        });
        let store = ConfigStore::in_dir(&dir);
        let mut config_error = None;
        let mut config = match store.load() {
            Ok(config) => config,
            Err(error) => {
                log::error!("configuration not loaded: {error}");
                config_error = Some(error.to_string());
                Config::default()
            }
        };

        if config.browsers.is_empty() && config_error.is_none() {
            let added = config.merge_discovered(signpost_platform::discover::installed_browsers());
            log::info!("first run: discovered {added} browser(s)");
            if let Err(error) = store.save(&config) {
                log::warn!("could not save the initial configuration: {error}");
            }
        }

        Self {
            store,
            config: Mutex::new(config),
            pending: Mutex::new(None),
            url_error: Mutex::new(None),
            config_error: Mutex::new(config_error),
        }
    }

    /// Read access to the configuration.
    pub fn config(&self) -> MutexGuard<'_, Config> {
        lock(&self.config)
    }

    /// Validate, persist and adopt a new configuration.
    pub fn save(&self, config: Config) -> Result<(), String> {
        self.store.save(&config).map_err(|e| e.to_string())?;
        *lock(&self.config) = config;
        *lock(&self.config_error) = None;
        Ok(())
    }

    /// Path of the configuration file, for display.
    pub fn config_path(&self) -> PathBuf {
        self.store.path().to_path_buf()
    }

    /// The URL waiting for a decision in the picker.
    pub fn pending(&self) -> Option<SafeUrl> {
        lock(&self.pending).clone()
    }

    /// Set the URL the picker should decide on, clearing any previous error.
    pub fn set_pending(&self, url: SafeUrl) {
        *lock(&self.pending) = Some(url);
        *lock(&self.url_error) = None;
    }

    /// Record why the incoming URL was refused (shown in the picker).
    pub fn set_url_error(&self, message: String) {
        *lock(&self.pending) = None;
        *lock(&self.url_error) = Some(message);
    }

    /// Last URL refusal, if any.
    pub fn url_error(&self) -> Option<String> {
        lock(&self.url_error).clone()
    }

    /// Why the configuration could not be loaded, if it could not.
    pub fn config_error(&self) -> Option<String> {
        lock(&self.config_error).clone()
    }
}
