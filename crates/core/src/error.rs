//! Error types. Every fallible operation in the crate returns one of these
//! so that callers (and the UI) can show a precise, non-technical message.

use thiserror::Error;

/// Why a URL was refused. Refusals are the normal, expected outcome for
/// anything that is not a plain `http`/`https` link.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum UrlError {
    /// The input is longer than [`crate::url::MAX_URL_LEN`] bytes.
    #[error("URL is too long ({0} bytes)")]
    TooLong(usize),
    /// The input contains ASCII control characters (e.g. newlines).
    #[error("URL contains control characters")]
    ControlCharacters,
    /// The input could not be parsed as a URL at all.
    #[error("not a valid URL: {0}")]
    Invalid(String),
    /// The scheme is not in the allow-list.
    #[error("scheme `{0}` is not allowed (only http and https)")]
    SchemeNotAllowed(String),
    /// The URL has no host part (e.g. `http:///path`).
    #[error("URL has no host")]
    MissingHost,
    /// The URL embeds a username or password, which Reroute refuses to
    /// forward because it is a well-known phishing vector.
    #[error("URLs with embedded credentials are refused")]
    Credentials,
}

/// Why a rule pattern could not be compiled.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PatternError {
    /// The pattern text is empty after trimming.
    #[error("pattern is empty")]
    Empty,
    /// The `kind:` prefix is unknown.
    #[error("unknown pattern kind `{0}` (expected exact, domain or regex)")]
    UnknownKind(String),
    /// A domain pattern contains characters that can never appear in a host.
    #[error("invalid domain pattern `{0}`")]
    InvalidDomain(String),
    /// The regular expression did not compile (message from the regex crate).
    #[error("invalid regex: {0}")]
    InvalidRegex(String),
}

/// Why a configuration document was rejected.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// TOML syntax or type error.
    #[error("cannot parse configuration: {0}")]
    Parse(String),
    /// The `version` field is newer than what this build understands.
    #[error("configuration version {found} is newer than supported version {supported}")]
    UnsupportedVersion {
        /// Version found in the file.
        found: u32,
        /// Highest version this build can read.
        supported: u32,
    },
    /// Two browsers or launches share the same id.
    #[error("duplicate id {0}")]
    DuplicateId(String),
    /// A browser has an empty name.
    #[error("browser {0} has an empty name")]
    EmptyBrowserName(String),
    /// A browser path is not absolute.
    #[error("browser `{name}` has a non-absolute path `{path}`")]
    RelativePath {
        /// Display name of the browser.
        name: String,
        /// The offending path.
        path: String,
    },
    /// A ruleset points at a browser id that does not exist.
    #[error("ruleset `{ruleset}` refers to unknown browser {browser}")]
    UnknownBrowser {
        /// Name of the ruleset.
        ruleset: String,
        /// The missing browser id.
        browser: String,
    },
    /// A ruleset points at a launch id that the browser does not have.
    #[error("ruleset `{ruleset}` refers to unknown launch {launch}")]
    UnknownLaunch {
        /// Name of the ruleset.
        ruleset: String,
        /// The missing launch id.
        launch: String,
    },
    /// A pattern inside a ruleset is invalid.
    #[error("ruleset `{ruleset}`: {source}")]
    Pattern {
        /// Name of the ruleset.
        ruleset: String,
        /// The underlying pattern error.
        #[source]
        source: PatternError,
    },
}

/// Why a browser could not be launched (checks performed before spawning).
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LaunchError {
    /// The browser id is not in the configuration.
    #[error("unknown browser {0}")]
    UnknownBrowser(String),
    /// The launch id does not belong to the chosen browser.
    #[error("unknown launch {0}")]
    UnknownLaunch(String),
    /// The path is not absolute.
    #[error("browser path `{0}` is not absolute")]
    RelativePath(String),
    /// The path does not exist or is not a regular file.
    #[error("browser executable `{0}` was not found")]
    NotFound(String),
    /// The file exists but is not executable.
    #[error("`{0}` is not executable")]
    NotExecutable(String),
    /// The OS refused to spawn the process.
    #[error("failed to start `{program}`: {reason}")]
    Spawn {
        /// Program that was attempted.
        program: String,
        /// OS error message.
        reason: String,
    },
}

/// Umbrella error for callers that do not need to distinguish causes.
#[derive(Debug, Error)]
pub enum Error {
    /// URL refused.
    #[error(transparent)]
    Url(#[from] UrlError),
    /// Pattern refused.
    #[error(transparent)]
    Pattern(#[from] PatternError),
    /// Configuration refused.
    #[error(transparent)]
    Config(#[from] ConfigError),
    /// Launch refused.
    #[error(transparent)]
    Launch(#[from] LaunchError),
    /// Underlying file-system error (reading or writing the configuration).
    #[error("I/O error on {path}: {source}")]
    Io {
        /// The file involved.
        path: String,
        /// OS error.
        #[source]
        source: std::io::Error,
    },
}
