//! Process-wide state shared between the command handlers.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::SystemTime;

use reroute_core::release::UpdateStatus;
use reroute_core::{Config, ConfigStore, SafeUrl};

/// Everything the windows need to know.
pub struct AppState {
    /// Where the configuration is read from and written to.
    pub store: ConfigStore,
    config: Mutex<Config>,
    /// Modification time of `config.toml` when it was last read or written,
    /// to notice an edit made by hand while Reroute runs in the background.
    read_at: Mutex<Option<SystemTime>>,
    pending: Mutex<Option<Pending>>,
    url_error: Mutex<Option<String>>,
    config_error: Mutex<Option<String>>,
    /// This process stays in the background for later clicks (Linux).
    resident: AtomicBool,
    /// A link waits for the picker, which shows once its page has read it.
    picker_wanted: AtomicBool,
    /// Where the running version stands against the newest release.
    update: Mutex<UpdateStatus>,
    /// An automatic version check is under way.
    checking: AtomicBool,
}

/// The link waiting for the picker.
#[derive(Clone)]
pub struct Pending {
    /// The link, validated.
    pub url: SafeUrl,
    /// The activation token the desktop gave its click, for the browser the
    /// user picks: it lets that browser come to the front.
    pub activation: Option<String>,
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
        let Some(dir) = reroute_platform::paths::config_dir().or_else(private_temporary_dir) else {
            log::error!("no directory to keep the configuration in, not even a temporary one");
            std::process::exit(1);
        };
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

        // First run: offer the installed browsers, and save them. With an
        // unreadable file, offer them too, so that links still open, but
        // save nothing: that file may hold rules the user wants back.
        if config.browsers.is_empty() {
            let added = config.merge_discovered(reroute_platform::discover::installed_browsers());
            if config_error.is_none() {
                log::info!("first run: discovered {added} browser(s)");
                if let Err(error) = store.save(&config) {
                    log::warn!("could not save the initial configuration: {error}");
                }
            } else {
                log::info!(
                    "offering {added} installed browser(s) until the configuration is repaired"
                );
            }
        }

        let read_at = modified(&store);
        Self {
            store,
            config: Mutex::new(config),
            read_at: Mutex::new(read_at),
            pending: Mutex::new(None),
            url_error: Mutex::new(None),
            config_error: Mutex::new(config_error),
            resident: AtomicBool::new(false),
            picker_wanted: AtomicBool::new(false),
            update: Mutex::new(UpdateStatus::Unknown),
            checking: AtomicBool::new(false),
        }
    }

    /// Read `config.toml` again if it changed since Reroute last read or
    /// wrote it: a Reroute running in the background must follow edits made
    /// by hand. A file that no longer parses is reported, and the rules in
    /// memory keep working meanwhile.
    #[cfg(target_os = "linux")]
    pub fn reload_if_changed(&self) {
        let now = modified(&self.store);
        if now == *lock(&self.read_at) {
            return;
        }
        match self.store.load() {
            Ok(config) => {
                *lock(&self.config) = config;
                *lock(&self.config_error) = None;
            }
            Err(error) => {
                log::error!("configuration not reloaded: {error}");
                *lock(&self.config_error) = Some(error.to_string());
            }
        }
        *lock(&self.read_at) = now;
    }

    /// Mark this process as the one that stays in the background.
    #[cfg(target_os = "linux")]
    pub fn set_resident(&self) {
        self.resident.store(true, Ordering::Relaxed);
    }

    /// Does this process keep running when its windows close? Only while
    /// it is the resident one and the setting still asks for it: turning
    /// the setting off lets Reroute exit when its windows close.
    pub fn stays_in_background(&self) -> bool {
        self.resident.load(Ordering::Relaxed) && self.config().settings.run_in_background
    }

    /// A link now waits for the picker.
    pub fn want_picker(&self) {
        self.picker_wanted.store(true, Ordering::Relaxed);
    }

    /// Was the picker asked for? Answers true once per request.
    pub fn take_picker_wish(&self) -> bool {
        self.picker_wanted.swap(false, Ordering::Relaxed)
    }

    /// Forget the link the picker was showing, once it is done.
    pub fn clear_pending(&self) {
        *lock(&self.pending) = None;
        *lock(&self.url_error) = None;
    }

    /// Where the running version stands.
    pub fn update_status(&self) -> UpdateStatus {
        lock(&self.update).clone()
    }

    /// Record the outcome of a version check.
    pub fn set_update_status(&self, status: UpdateStatus) {
        *lock(&self.update) = status;
    }

    /// Start an automatic check, unless one is already under way.
    pub fn begin_update_check(&self) -> bool {
        !self.checking.swap(true, Ordering::Relaxed)
    }

    /// The automatic check has finished.
    pub fn end_update_check(&self) {
        self.checking.store(false, Ordering::Relaxed);
    }

    /// Read access to the configuration.
    pub fn config(&self) -> MutexGuard<'_, Config> {
        lock(&self.config)
    }

    /// Validate, persist and adopt a new configuration.
    ///
    /// If the file could not be read at start-up, it is moved aside as
    /// `config.toml.broken` first: the user may want the rules it holds, and
    /// the empty configuration in memory must not overwrite them.
    pub fn save(&self, config: Config) -> Result<(), String> {
        if self.config_error().is_some()
            && let Some(kept) = self.store.set_aside().map_err(|e| e.to_string())?
        {
            log::warn!("unreadable configuration kept as {}", kept.display());
        }
        self.store.save(&config).map_err(|e| e.to_string())?;
        *lock(&self.config) = config;
        *lock(&self.config_error) = None;
        *lock(&self.read_at) = modified(&self.store);
        Ok(())
    }

    /// Path of the configuration file, for display.
    pub fn config_path(&self) -> PathBuf {
        self.store.path().to_path_buf()
    }

    /// The link waiting for a decision in the picker.
    pub fn pending(&self) -> Option<Pending> {
        lock(&self.pending).clone()
    }

    /// Set the link the picker should decide on, clearing any previous
    /// error.
    pub fn set_pending(&self, url: SafeUrl, activation: Option<String>) {
        *lock(&self.pending) = Some(Pending { url, activation });
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

/// Where to keep the configuration when the system offers no configuration
/// directory (there is no home directory): a new directory under a random
/// name, which only this user can open. It lasts one run. A fixed name in
/// the shared temporary directory would let another user create it first,
/// with a configuration that starts programs of their choosing.
fn private_temporary_dir() -> Option<PathBuf> {
    log::warn!("no configuration directory; the settings will last this run only");
    tempfile::Builder::new()
        .prefix("reroute-")
        .tempdir()
        .map(tempfile::TempDir::keep)
        .map_err(|error| log::error!("no temporary directory either: {error}"))
        .ok()
}

/// When `config.toml` was last modified, if it exists.
fn modified(store: &ConfigStore) -> Option<SystemTime> {
    std::fs::metadata(store.path())
        .and_then(|meta| meta.modified())
        .ok()
}
