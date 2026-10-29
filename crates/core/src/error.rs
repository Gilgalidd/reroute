//! Error types. Every fallible operation in the crate returns one of these
//! so that callers (and the UI) can show a precise, non-technical message.
//!
//! Each one's `Display` is the English text for logs and the command line;
//! its `message()` is what the interface translates (see [`crate::message`]).
//! `message()` is an exhaustive `match`: a new variant needs a code.

use thiserror::Error;

use crate::message::{self, Message};

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
    /// The `language` setting is neither `auto` nor a language tag.
    #[error("language `{0}` is not a language tag such as fr or pt-BR")]
    InvalidLanguage(String),
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

/// Why a Hurl settings file could not be imported.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ImportError {
    /// The text is not JSON shaped like Hurl's `UserSettings.json`.
    #[error("not a Hurl settings file: {0}")]
    NotHurl(String),
    /// What it converts to is not a valid configuration.
    #[error(transparent)]
    Config(#[from] ConfigError),
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

impl UrlError {
    /// What to tell the user.
    pub fn message(&self) -> Message {
        let text = self.to_string();
        match self {
            Self::TooLong(bytes) => message::URL_TOO_LONG.with([(*bytes).into()], text),
            Self::ControlCharacters => message::URL_CONTROL_CHARACTERS.with([], text),
            Self::Invalid(detail) => message::URL_INVALID.with([detail.as_str().into()], text),
            Self::SchemeNotAllowed(scheme) => {
                message::URL_SCHEME_NOT_ALLOWED.with([scheme.as_str().into()], text)
            }
            Self::MissingHost => message::URL_MISSING_HOST.with([], text),
            Self::Credentials => message::URL_CREDENTIALS.with([], text),
        }
    }
}

impl PatternError {
    /// What to tell the user.
    pub fn message(&self) -> Message {
        let text = self.to_string();
        match self {
            Self::Empty => message::PATTERN_EMPTY.with([], text),
            Self::UnknownKind(kind) => {
                message::PATTERN_UNKNOWN_KIND.with([kind.as_str().into()], text)
            }
            Self::InvalidDomain(pattern) => {
                message::PATTERN_INVALID_DOMAIN.with([pattern.as_str().into()], text)
            }
            Self::InvalidRegex(detail) => {
                message::PATTERN_INVALID_REGEX.with([detail.as_str().into()], text)
            }
        }
    }
}

impl ConfigError {
    /// What to tell the user.
    pub fn message(&self) -> Message {
        let text = self.to_string();
        match self {
            Self::Parse(detail) => message::CONFIG_PARSE.with([detail.as_str().into()], text),
            Self::UnsupportedVersion { found, supported } => message::CONFIG_UNSUPPORTED_VERSION
                .with([(*found).into(), (*supported).into()], text),
            Self::DuplicateId(id) => message::CONFIG_DUPLICATE_ID.with([id.as_str().into()], text),
            Self::EmptyBrowserName(id) => {
                message::CONFIG_EMPTY_BROWSER_NAME.with([id.as_str().into()], text)
            }
            Self::RelativePath { name, path } => message::CONFIG_RELATIVE_PATH
                .with([name.as_str().into(), path.as_str().into()], text),
            Self::UnknownBrowser { ruleset, browser } => message::CONFIG_UNKNOWN_BROWSER
                .with([ruleset.as_str().into(), browser.as_str().into()], text),
            Self::UnknownLaunch { ruleset, launch } => message::CONFIG_UNKNOWN_LAUNCH
                .with([ruleset.as_str().into(), launch.as_str().into()], text),
            Self::InvalidLanguage(language) => {
                message::CONFIG_INVALID_LANGUAGE.with([language.as_str().into()], text)
            }
            Self::Pattern { ruleset, source } => message::CONFIG_PATTERN
                .with([ruleset.as_str().into(), source.message().into()], text),
        }
    }
}

impl LaunchError {
    /// What to tell the user.
    pub fn message(&self) -> Message {
        let text = self.to_string();
        match self {
            Self::UnknownBrowser(browser) => {
                message::LAUNCH_UNKNOWN_BROWSER.with([browser.as_str().into()], text)
            }
            Self::UnknownLaunch(launch) => {
                message::LAUNCH_UNKNOWN_LAUNCH.with([launch.as_str().into()], text)
            }
            Self::RelativePath(path) => {
                message::LAUNCH_RELATIVE_PATH.with([path.as_str().into()], text)
            }
            Self::NotFound(path) => message::LAUNCH_NOT_FOUND.with([path.as_str().into()], text),
            Self::NotExecutable(path) => {
                message::LAUNCH_NOT_EXECUTABLE.with([path.as_str().into()], text)
            }
            Self::Spawn { program, reason } => {
                message::LAUNCH_SPAWN.with([program.as_str().into(), reason.as_str().into()], text)
            }
        }
    }
}

impl ImportError {
    /// What to tell the user.
    pub fn message(&self) -> Message {
        match self {
            Self::NotHurl(detail) => {
                message::HURL_NOT_HURL.with([detail.as_str().into()], self.to_string())
            }
            Self::Config(error) => error.message(),
        }
    }
}

impl Error {
    /// What to tell the user.
    pub fn message(&self) -> Message {
        match self {
            Self::Url(error) => error.message(),
            Self::Pattern(error) => error.message(),
            Self::Config(error) => error.message(),
            Self::Launch(error) => error.message(),
            Self::Io { path, source } => message::FILE.with(
                [path.as_str().into(), source.to_string().into()],
                self.to_string(),
            ),
        }
    }
}

// So that `?` turns an error into the message a command returns.
impl From<UrlError> for Message {
    fn from(error: UrlError) -> Self {
        error.message()
    }
}

impl From<PatternError> for Message {
    fn from(error: PatternError) -> Self {
        error.message()
    }
}

impl From<ConfigError> for Message {
    fn from(error: ConfigError) -> Self {
        error.message()
    }
}

impl From<LaunchError> for Message {
    fn from(error: LaunchError) -> Self {
        error.message()
    }
}

impl From<ImportError> for Message {
    fn from(error: ImportError) -> Self {
        error.message()
    }
}

impl From<Error> for Message {
    fn from(error: Error) -> Self {
        error.message()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Param;

    #[test]
    fn each_error_keeps_its_values_and_its_english_text() {
        let refused = UrlError::SchemeNotAllowed("ftp".into()).message();
        assert_eq!(refused.code, "url.scheme_not_allowed");
        assert_eq!(refused.params["scheme"], Param::Text("ftp".into()));
        assert_eq!(
            refused.text,
            "scheme `ftp` is not allowed (only http and https)"
        );

        let newer = ConfigError::UnsupportedVersion {
            found: 3,
            supported: 1,
        }
        .message();
        assert_eq!(newer.params["found"], Param::Number(3));
        assert_eq!(newer.params["supported"], Param::Number(1));

        let spawn = LaunchError::Spawn {
            program: "/usr/bin/firefox".into(),
            reason: "Permission denied".into(),
        }
        .message();
        assert_eq!(
            spawn.params["detail"],
            Param::Text("Permission denied".into())
        );
    }

    #[test]
    fn a_reason_inside_a_reason_is_a_message_of_its_own() {
        let error = Error::Config(ConfigError::Pattern {
            ruleset: "Work".into(),
            source: PatternError::InvalidDomain("a b".into()),
        });
        let message = error.message();
        assert_eq!(message.code, "config.pattern");
        assert_eq!(message.text, "ruleset `Work`: invalid domain pattern `a b`");
        let Param::Message(problem) = &message.params["problem"] else {
            panic!("{message:?}");
        };
        assert_eq!(problem.code, "pattern.invalid_domain");
        assert_eq!(problem.params["pattern"], Param::Text("a b".into()));
    }

    #[test]
    fn a_file_error_names_the_file_and_keeps_the_system_s_words() {
        let error = Error::Io {
            path: "/home/me/.config/reroute/config.toml".into(),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Permission denied"),
        };
        let message = Message::from(error);
        assert_eq!(message.code, "file");
        assert_eq!(
            message.params["path"],
            Param::Text("/home/me/.config/reroute/config.toml".into())
        );
        assert_eq!(
            message.params["detail"],
            Param::Text("Permission denied".into())
        );
    }

    #[test]
    fn an_import_error_says_what_is_wrong() {
        assert_eq!(
            ImportError::NotHurl("expected value".into()).message().code,
            "hurl.not_hurl"
        );
        let invalid = ImportError::from(ConfigError::DuplicateId("x".into()));
        assert_eq!(invalid.message().code, "config.duplicate_id");
    }
}
