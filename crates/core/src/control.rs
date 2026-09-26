//! What a new Reroute process asks the one already running in the
//! background.
//!
//! On Linux, Reroute can keep running with its picker loaded, so that the
//! picker opens at once. A click still starts a new `reroute <url>`
//! process, since that is how the system opens links; that process hands
//! its request over a local socket to the running Reroute and exits. The
//! protocol is one line of text in each direction, small enough to check at
//! a glance: `open <url>` or `settings`, answered by `ok`.
//!
//! The URL is not validated here: the running Reroute passes it through
//! [`crate::SafeUrl`] exactly as it would a URL from its own command line.

use crate::url::MAX_URL_LEN;

/// Longest request accepted, newline included: a URL of the longest
/// accepted length plus the command word.
pub const MAX_REQUEST: usize = MAX_URL_LEN + 16;

/// The answer to a request that was understood.
pub const OK: &str = "ok";

/// A request from another Reroute process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Route this link, as `reroute <url>` would.
    Open(String),
    /// Show the settings window, as `reroute` alone would.
    Settings,
}

/// Why a request line was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ControlError {
    /// The line is longer than [`MAX_REQUEST`].
    #[error("request too long")]
    TooLong,
    /// The line holds a control character, which would break the framing.
    #[error("request contains control characters")]
    ControlCharacters,
    /// Neither `open <url>` nor `settings`.
    #[error("unknown request")]
    Unknown,
}

impl Request {
    /// The line that carries this request, newline included.
    pub fn encode(&self) -> Result<String, ControlError> {
        let line = match self {
            Self::Open(url) => format!("open {url}"),
            Self::Settings => "settings".to_owned(),
        };
        check(&line)?;
        Ok(line + "\n")
    }

    /// Read a request line; the trailing newline is optional.
    pub fn decode(line: &str) -> Result<Self, ControlError> {
        let line = line.strip_suffix('\n').unwrap_or(line);
        check(line)?;
        match line {
            "settings" => Ok(Self::Settings),
            _ => match line.strip_prefix("open ") {
                Some(url) if !url.is_empty() => Ok(Self::Open(url.to_owned())),
                _ => Err(ControlError::Unknown),
            },
        }
    }
}

fn check(line: &str) -> Result<(), ControlError> {
    if line.len() >= MAX_REQUEST {
        return Err(ControlError::TooLong);
    }
    if line.chars().any(char::is_control) {
        return Err(ControlError::ControlCharacters);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn both_requests_round_trip() {
        for request in [
            Request::Open("https://example.com/a?b=c d".into()),
            Request::Settings,
        ] {
            let line = request.encode().unwrap();
            assert!(line.ends_with('\n'));
            assert_eq!(Request::decode(&line).unwrap(), request);
        }
    }

    #[test]
    fn anything_else_is_refused() {
        assert_eq!(Request::decode("open "), Err(ControlError::Unknown));
        assert_eq!(Request::decode("quit"), Err(ControlError::Unknown));
        assert_eq!(Request::decode(""), Err(ControlError::Unknown));
        assert_eq!(
            Request::decode("open https://a.org/\u{0}x"),
            Err(ControlError::ControlCharacters)
        );
        let long = format!("open https://a.org/{}", "x".repeat(MAX_URL_LEN));
        assert_eq!(Request::decode(&long), Err(ControlError::TooLong));
    }

    #[test]
    fn a_url_that_would_break_the_line_is_not_sent() {
        assert_eq!(
            Request::Open("https://a.org/\nsettings".into()).encode(),
            Err(ControlError::ControlCharacters)
        );
    }

    proptest! {
        /// Whatever arrives on the socket, decoding never panics, and what
        /// it accepts is one of the two requests.
        #[test]
        fn decoding_is_total(s in "\\PC{0,200}") {
            let _ = Request::decode(&s);
        }

        #[test]
        fn open_round_trips(url in "[a-zA-Z0-9:/?#.=&%~_-]{1,200}") {
            let request = Request::Open(url);
            prop_assert_eq!(Request::decode(&request.encode().unwrap()).unwrap(), request);
        }
    }
}
