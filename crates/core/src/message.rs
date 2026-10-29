//! What Reroute tells a person, in a form the interface can translate.
//!
//! A [`Message`] names what happened with a code, such as
//! `url.scheme_not_allowed`, and carries the values its sentence needs, such
//! as the scheme. The interface words it in the user's language, from the
//! key `rust.<code>` of its catalogue. `text` is the English sentence: logs
//! and the command line print it, and the interface shows it for a code it
//! does not know. See docs/translations.md.
//!
//! Every code is declared with [`message_codes!`](crate::message_codes),
//! which also lists them: the desktop application checks that list against
//! the one the interface translates, `apps/desktop/src/i18n/rust-messages.json`.

use std::collections::BTreeMap;
use std::fmt;

use serde::Serialize;

/// A kind of message: its code and the names of its `N` parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Code<const N: usize> {
    /// Looked up as `rust.<code>` in the interface's catalogue.
    pub code: &'static str,
    /// Names of the values the message carries, in the order
    /// [`Code::with`] takes them.
    pub params: [&'static str; N],
}

impl<const N: usize> Code<N> {
    /// A code and the names of its parameters.
    pub const fn new(code: &'static str, params: [&'static str; N]) -> Self {
        Self { code, params }
    }

    /// The message with `values` for the parameters, in their order, and
    /// `text`, its English sentence.
    pub fn with(&self, values: [Param; N], text: impl Into<String>) -> Message {
        Message {
            code: self.code,
            params: self.params.into_iter().zip(values).collect(),
            text: text.into(),
        }
    }
}

/// A value inside a message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Param {
    /// Shown as it is: a name, a path, or a third party's message, which
    /// stays in English.
    Text(String),
    /// A number, which the interface writes the way its language does.
    Number(u64),
    /// Another message, worded in turn: the reason inside a reason.
    Message(Box<Message>),
}

impl From<String> for Param {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<&str> for Param {
    fn from(text: &str) -> Self {
        Self::Text(text.to_owned())
    }
}

impl From<u64> for Param {
    fn from(number: u64) -> Self {
        Self::Number(number)
    }
}

impl From<u32> for Param {
    fn from(number: u32) -> Self {
        Self::Number(number.into())
    }
}

impl From<usize> for Param {
    fn from(number: usize) -> Self {
        Self::Number(u64::try_from(number).unwrap_or(u64::MAX))
    }
}

impl From<Message> for Param {
    fn from(message: Message) -> Self {
        Self::Message(Box::new(message))
    }
}

/// Something to tell the user, sent to the interface as
/// `{ "code", "params", "text" }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Message {
    /// What happened; see [`Code`].
    pub code: &'static str,
    /// The values the sentence needs, by name.
    pub params: BTreeMap<&'static str, Param>,
    /// The English sentence, for logs and for an interface that does not
    /// know `code`.
    pub text: String,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

/// Declare message codes. Each line `NAME = "code" (param, …);` becomes a
/// [`Code`] constant with that many parameters, and `CODES` lists them all,
/// for the check against the interface's list.
#[macro_export]
macro_rules! message_codes {
    ($($name:ident = $code:literal $(($($param:ident),+))?;)+) => {
        $(
            #[doc = concat!("The message `", $code, "`.")]
            pub const $name: $crate::message::Code<
                { <[&'static str]>::len(&[$($(stringify!($param)),+)?]) },
            > = $crate::message::Code::new($code, [$($(stringify!($param)),+)?]);
        )+

        /// Every code declared here, with the names of its parameters.
        #[allow(dead_code)]
        pub const CODES: &[(&str, &[&str])] = &[$(($name.code, &$name.params)),+];
    };
}

// The codes of this crate's errors and notes.
message_codes! {
    URL_TOO_LONG = "url.too_long" (bytes);
    URL_CONTROL_CHARACTERS = "url.control_characters";
    URL_INVALID = "url.invalid" (detail);
    URL_SCHEME_NOT_ALLOWED = "url.scheme_not_allowed" (scheme);
    URL_MISSING_HOST = "url.missing_host";
    URL_CREDENTIALS = "url.credentials";

    PATTERN_EMPTY = "pattern.empty";
    PATTERN_UNKNOWN_KIND = "pattern.unknown_kind" (kind);
    PATTERN_INVALID_DOMAIN = "pattern.invalid_domain" (pattern);
    PATTERN_INVALID_REGEX = "pattern.invalid_regex" (detail);

    CONFIG_PARSE = "config.parse" (detail);
    CONFIG_UNSUPPORTED_VERSION = "config.unsupported_version" (found, supported);
    CONFIG_DUPLICATE_ID = "config.duplicate_id" (id);
    CONFIG_EMPTY_BROWSER_NAME = "config.empty_browser_name" (id);
    CONFIG_RELATIVE_PATH = "config.relative_path" (name, path);
    CONFIG_UNKNOWN_BROWSER = "config.unknown_browser" (ruleset, browser);
    CONFIG_UNKNOWN_LAUNCH = "config.unknown_launch" (ruleset, launch);
    CONFIG_INVALID_LANGUAGE = "config.invalid_language" (language);
    CONFIG_PATTERN = "config.pattern" (ruleset, problem);

    LAUNCH_UNKNOWN_BROWSER = "launch.unknown_browser" (browser);
    LAUNCH_UNKNOWN_LAUNCH = "launch.unknown_launch" (launch);
    LAUNCH_RELATIVE_PATH = "launch.relative_path" (path);
    LAUNCH_NOT_FOUND = "launch.not_found" (path);
    LAUNCH_NOT_EXECUTABLE = "launch.not_executable" (path);
    LAUNCH_SPAWN = "launch.spawn" (program, detail);

    FILE = "file" (path, detail);

    MERGE_BROWSER_EXISTS = "merge.browser_exists" (browser, existing);
    MERGE_UNKNOWN_BROWSER = "merge.unknown_browser" (ruleset);
    MERGE_DEFAULT_LAUNCH = "merge.default_launch" (ruleset);

    HURL_NOT_HURL = "hurl.not_hurl" (detail);
    HURL_UWP = "hurl.uwp" (browser);
    HURL_RELATIVE_PATH = "hurl.relative_path" (browser, path);
    HURL_REMOTE_ICON = "hurl.remote_icon" (browser, icon);
    HURL_UNKNOWN_BROWSER = "hurl.unknown_browser" (ruleset, browser);
    HURL_BAD_RULE = "hurl.bad_rule" (rule, ruleset, problem);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_names_its_values() {
        let message = URL_SCHEME_NOT_ALLOWED.with(["ftp".into()], "scheme `ftp` is not allowed");
        assert_eq!(message.code, "url.scheme_not_allowed");
        assert_eq!(message.params["scheme"], Param::Text("ftp".into()));
        assert_eq!(message.to_string(), "scheme `ftp` is not allowed");
    }

    #[test]
    fn it_reaches_the_interface_as_code_params_and_text() {
        let inner = PATTERN_EMPTY.with([], "pattern is empty");
        let outer = CONFIG_PATTERN.with(
            ["Work".into(), inner.into()],
            "ruleset `Work`: pattern is empty",
        );
        let json = serde_json::to_value(&outer).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "code": "config.pattern",
                "params": {
                    "ruleset": "Work",
                    "problem": { "code": "pattern.empty", "params": {}, "text": "pattern is empty" },
                },
                "text": "ruleset `Work`: pattern is empty",
            })
        );
        let long = URL_TOO_LONG.with([9000_usize.into()], "URL is too long (9000 bytes)");
        assert_eq!(
            serde_json::to_value(&long).unwrap()["params"]["bytes"],
            9000
        );
    }

    #[test]
    fn every_code_is_declared_once_with_its_parameters() {
        let mut codes: Vec<&str> = CODES.iter().map(|(code, _)| *code).collect();
        let count = codes.len();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), count, "a code declared twice");
        assert!(CODES.contains(&("config.unsupported_version", &["found", "supported"][..])));
        assert!(CODES.contains(&("url.missing_host", &[][..])));
    }
}
