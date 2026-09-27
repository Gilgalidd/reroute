//! Process-wide state shared between the command handlers.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::SystemTime;

use reroute_core::release::UpdateStatus;
use reroute_core::{Browser, Config, ConfigStore, SafeUrl};

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
        Self::open(
            ConfigStore::in_dir(&dir),
            reroute_platform::discover::installed_browsers,
        )
    }

    /// [`load`](Self::load) from `store`. `installed` lists the browsers on
    /// the computer; it is called only when the configuration has none, and
    /// tests pass their own.
    pub(crate) fn open(store: ConfigStore, installed: impl FnOnce() -> Vec<Browser>) -> Self {
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
            let added = config.merge_discovered(installed());
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

    /// The link waiting for the picker, if it is still `url`, the one the
    /// picker showed when the user chose: a new link may have arrived
    /// meanwhile, and the choice, "always use for this domain" included,
    /// was made for the link on screen.
    pub fn pending_for(&self, url: &str) -> Result<Pending, &'static str> {
        match self.pending() {
            None => Err("there is no URL to open"),
            Some(pending) if pending.url.as_str() == url => Ok(pending),
            Some(_) => Err("A new link arrived. Choose a browser for it."),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// An absolute path that exists on every system. It is only checked,
    /// never started.
    fn some_program() -> PathBuf {
        std::env::current_exe().unwrap()
    }

    fn installed() -> Vec<Browser> {
        vec![Browser::new("Firefox", some_program())]
    }

    fn url(text: &str) -> SafeUrl {
        SafeUrl::parse(text).unwrap()
    }

    #[test]
    fn the_first_run_saves_the_browsers_it_finds() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::open(ConfigStore::in_dir(dir.path()), installed);
        assert_eq!(state.config().browsers.len(), 1);
        assert_eq!(state.config_error(), None);
        assert_eq!(state.store.load().unwrap().browsers.len(), 1, "saved");
    }

    #[test]
    fn an_unreadable_file_offers_the_browsers_and_is_kept_until_save() {
        let dir = tempfile::tempdir().unwrap();
        let store = ConfigStore::in_dir(dir.path());
        std::fs::write(store.path(), "[settings\nbroken").unwrap();
        let state = AppState::open(store.clone(), installed);
        assert!(state.config_error().is_some());
        assert_eq!(state.config().browsers.len(), 1, "links still open");
        assert_eq!(
            std::fs::read_to_string(store.path()).unwrap(),
            "[settings\nbroken",
            "nothing written before Save"
        );

        let config = state.config().clone();
        state.save(config).unwrap();
        assert!(store.path().with_extension("toml.broken").exists());
        assert!(store.load().is_ok());
        assert_eq!(state.config_error(), None);
    }

    #[test]
    fn a_choice_counts_only_for_the_link_on_screen() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::open(ConfigStore::in_dir(dir.path()), installed);
        assert!(state.pending_for("https://a.org/").is_err(), "no link yet");

        state.set_pending(url("https://a.org/"), Some("kwin-1".into()));
        let pending = state.pending_for("https://a.org/").unwrap();
        assert_eq!(pending.activation.as_deref(), Some("kwin-1"));

        state.set_pending(url("https://b.org/"), None);
        assert!(state.pending_for("https://a.org/").is_err(), "replaced");
    }

    #[test]
    fn a_refused_link_replaces_the_pending_one_until_the_picker_is_done() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::open(ConfigStore::in_dir(dir.path()), installed);
        state.set_pending(url("https://a.org/"), None);
        state.set_url_error("scheme `javascript` is not allowed".into());
        assert!(state.pending().is_none());
        assert!(state.url_error().is_some());
        state.clear_pending();
        assert!(state.pending().is_none());
        assert!(state.url_error().is_none());
    }

    #[test]
    fn the_picker_shows_once_per_request() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::open(ConfigStore::in_dir(dir.path()), installed);
        assert!(!state.take_picker_wish());
        state.want_picker();
        assert!(state.take_picker_wish());
        assert!(
            !state.take_picker_wish(),
            "the page and the safety timer cannot both show it"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn staying_in_the_background_follows_the_setting() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::open(ConfigStore::in_dir(dir.path()), installed);
        assert!(!state.stays_in_background(), "not the resident one");
        state.set_resident();
        assert!(state.stays_in_background());
        let mut config = state.config().clone();
        config.settings.run_in_background = false;
        state.save(config).unwrap();
        assert!(
            !state.stays_in_background(),
            "turned off: exits with its windows"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn an_edit_by_hand_is_read_again() {
        use std::time::Duration;
        let dir = tempfile::tempdir().unwrap();
        let store = ConfigStore::in_dir(dir.path());
        let state = AppState::open(store.clone(), installed);
        let edit = |text: String, later: u64| {
            std::fs::write(store.path(), text).unwrap();
            std::fs::File::options()
                .write(true)
                .open(store.path())
                .unwrap()
                .set_modified(SystemTime::now() + Duration::from_secs(later))
                .unwrap();
        };

        let text = std::fs::read_to_string(store.path()).unwrap();
        edit(text.replace("\"Firefox\"", "\"Firefox, by hand\""), 10);
        state.reload_if_changed();
        assert_eq!(state.config().browsers[0].name, "Firefox, by hand");

        // A broken edit keeps the rules in memory and says why.
        edit("broken [".into(), 20);
        state.reload_if_changed();
        assert_eq!(state.config().browsers[0].name, "Firefox, by hand");
        assert!(state.config_error().is_some());
    }
}
