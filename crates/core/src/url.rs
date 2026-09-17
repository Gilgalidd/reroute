//! Strict URL validation.
//!
//! The URL is the only input that arrives from outside the user's control
//! (any application can ask the OS to "open" a string). [`SafeUrl`] is the
//! single gate through which such a string must pass before Signpost will
//! match rules against it or hand it to a browser.

use std::fmt;

use serde::Serialize;

use crate::error::UrlError;

/// Upper bound on the accepted input size. Real links are a few hundred
/// bytes; the limit protects command-line length limits and regex matching
/// time.
pub const MAX_URL_LEN: usize = 8 * 1024;

/// Schemes that Signpost is willing to forward. The list is intentionally
/// not configurable: `javascript:`, `file:`, `data:` and friends must never
/// reach a browser through a picker that pretends to be a browser.
pub const ALLOWED_SCHEMES: &[&str] = &["http", "https"];

/// A URL that passed every validation rule in this module.
///
/// The type is immutable; the only way to obtain one is [`SafeUrl::parse`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SafeUrl {
    #[serde(skip)]
    original: String,
    normalized: url::Url,
}

impl SafeUrl {
    /// Validate and normalise `input`.
    ///
    /// Leading and trailing whitespace is tolerated (shells and launchers
    /// often add it). Everything else is strict; see [`UrlError`].
    pub fn parse(input: &str) -> Result<Self, UrlError> {
        let trimmed = input.trim();
        if trimmed.len() > MAX_URL_LEN {
            return Err(UrlError::TooLong(trimmed.len()));
        }
        if trimmed.chars().any(char::is_control) {
            return Err(UrlError::ControlCharacters);
        }
        let parsed = url::Url::parse(trimmed).map_err(|e| UrlError::Invalid(e.to_string()))?;
        if !ALLOWED_SCHEMES.contains(&parsed.scheme()) {
            return Err(UrlError::SchemeNotAllowed(parsed.scheme().to_owned()));
        }
        if parsed.host_str().is_none_or(str::is_empty) {
            return Err(UrlError::MissingHost);
        }
        if !parsed.username().is_empty() || parsed.password().is_some() {
            return Err(UrlError::Credentials);
        }
        Ok(Self {
            original: trimmed.to_owned(),
            normalized: parsed,
        })
    }

    /// The normalised form (lower-case scheme and host, percent-encoding
    /// applied). This is what gets passed to the browser.
    pub fn as_str(&self) -> &str {
        self.normalized.as_str()
    }

    /// The input exactly as received (after trimming). Used so that
    /// `exact:` rules written by hand keep matching.
    pub fn original(&self) -> &str {
        &self.original
    }

    /// Lower-case host, without port. Guaranteed non-empty.
    pub fn host(&self) -> &str {
        self.normalized.host_str().unwrap_or_default()
    }

    /// The scheme (`http` or `https`).
    pub fn scheme(&self) -> &str {
        self.normalized.scheme()
    }
}

impl fmt::Display for SafeUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn accepts_plain_https() {
        let u = SafeUrl::parse("https://Example.COM/path?q=1#frag").unwrap();
        assert_eq!(u.as_str(), "https://example.com/path?q=1#frag");
        assert_eq!(u.host(), "example.com");
        assert_eq!(u.scheme(), "https");
    }

    #[test]
    fn trims_whitespace_but_keeps_original() {
        let u = SafeUrl::parse("  http://a.org \n").unwrap();
        assert_eq!(u.original(), "http://a.org");
        assert_eq!(u.as_str(), "http://a.org/");
    }

    #[test]
    fn rejects_other_schemes() {
        for bad in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "data:text/html,x",
            "ftp://x.org",
            "mailto:a@b.c",
        ] {
            assert!(
                matches!(SafeUrl::parse(bad), Err(UrlError::SchemeNotAllowed(_))),
                "{bad}"
            );
        }
    }

    #[test]
    fn rejects_credentials() {
        assert_eq!(
            SafeUrl::parse("https://user:pw@example.com/").unwrap_err(),
            UrlError::Credentials
        );
        assert_eq!(
            SafeUrl::parse("https://user@example.com/").unwrap_err(),
            UrlError::Credentials
        );
    }

    #[test]
    fn rejects_missing_host_and_garbage() {
        assert!(matches!(
            SafeUrl::parse("http://"),
            Err(UrlError::Invalid(_))
        ));
        assert!(matches!(
            SafeUrl::parse("not a url"),
            Err(UrlError::Invalid(_))
        ));
        assert!(matches!(SafeUrl::parse(""), Err(UrlError::Invalid(_))));
    }

    #[test]
    fn rejects_control_chars_and_length() {
        assert_eq!(
            SafeUrl::parse("https://a.org/\u{7}").unwrap_err(),
            UrlError::ControlCharacters
        );
        assert_eq!(
            SafeUrl::parse("https://a.org/x\ny").unwrap_err(),
            UrlError::ControlCharacters
        );
        let long = format!("https://a.org/{}", "x".repeat(MAX_URL_LEN));
        assert!(matches!(SafeUrl::parse(&long), Err(UrlError::TooLong(_))));
    }

    #[test]
    fn idn_hosts_are_normalised_to_punycode() {
        let u = SafeUrl::parse("https://bücher.example/").unwrap();
        assert_eq!(u.host(), "xn--bcher-kva.example");
    }

    proptest! {
        /// Whatever we accept must start with an allowed scheme and never
        /// contain control characters: a browser can therefore never be
        /// tricked into interpreting the argument as an option.
        #[test]
        fn accepted_urls_are_well_formed(s in "\\PC{0,300}") {
            if let Ok(u) = SafeUrl::parse(&s) {
                prop_assert!(u.as_str().starts_with("http://") || u.as_str().starts_with("https://"));
                prop_assert!(!u.as_str().chars().any(char::is_control));
                prop_assert!(!u.host().is_empty());
                prop_assert!(!u.as_str().starts_with('-'));
            }
        }

        #[test]
        fn parsing_is_idempotent(s in "https?://[a-z0-9.-]{1,20}(/[a-zA-Z0-9._~%-]{0,20})?") {
            if let Ok(u) = SafeUrl::parse(&s) {
                let again = SafeUrl::parse(u.as_str()).unwrap();
                prop_assert_eq!(again.as_str(), u.as_str());
            }
        }
    }
}
