//! What a new Reroute process asks the one already running in the
//! background.
//!
//! On Linux, Reroute can keep running with its picker loaded, so that the
//! picker opens at once. A click still starts a new `reroute <url>`
//! process, since that is how the system opens links; that process hands
//! its request over a local socket to the running Reroute and exits. The
//! protocol is one line of text in each direction, small enough to check at
//! a glance: `open <url>`, `open-activated <token> <url>` or `settings`,
//! answered by `ok`.
//!
//! The token is the activation token the desktop gave the click
//! (`XDG_ACTIVATION_TOKEN` on Wayland). The browser opened for that click
//! needs it to come to the front, and the running Reroute cannot use its
//! own: that one belonged to an earlier click and has been used.
//!
//! The URL is not validated here: the running Reroute passes it through
//! [`crate::SafeUrl`] exactly as it would a URL from its own command line.

use crate::url::MAX_URL_LEN;

/// Longest activation token passed on. Real ones are a few dozen bytes.
pub const MAX_ACTIVATION_TOKEN: usize = 256;

/// Longest request accepted, newline included: a URL and a token of the
/// longest accepted lengths, plus the command word.
pub const MAX_REQUEST: usize = MAX_URL_LEN + MAX_ACTIVATION_TOKEN + 32;

/// The answer to a request that was understood.
pub const OK: &str = "ok";

/// A request from another Reroute process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Route this link, as `reroute <url>` would.
    Open {
        /// The link, not validated yet.
        url: String,
        /// The activation token of the click, for the browser; see
        /// [`is_activation_token`].
        activation: Option<String>,
    },
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
    /// The activation token is not one word of printable ASCII.
    #[error("invalid activation token")]
    Token,
    /// Neither `open <url>`, `open-activated <token> <url>` nor `settings`.
    #[error("unknown request")]
    Unknown,
}

/// Can `token` travel as an activation token? One word of printable ASCII,
/// no longer than [`MAX_ACTIVATION_TOKEN`]: the desktops' tokens look like
/// `kwin-12`. Anything else is not passed on.
pub fn is_activation_token(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= MAX_ACTIVATION_TOKEN
        && token.bytes().all(|b| b.is_ascii_graphic())
}

impl Request {
    /// The line that carries this request, newline included.
    pub fn encode(&self) -> Result<String, ControlError> {
        let line = match self {
            Self::Open {
                url,
                activation: None,
            } => format!("open {url}"),
            Self::Open {
                url,
                activation: Some(token),
            } => {
                if !is_activation_token(token) {
                    return Err(ControlError::Token);
                }
                format!("open-activated {token} {url}")
            }
            Self::Settings => "settings".to_owned(),
        };
        check(&line)?;
        Ok(line + "\n")
    }

    /// Read a request line; the trailing newline is optional.
    pub fn decode(line: &str) -> Result<Self, ControlError> {
        let line = line.strip_suffix('\n').unwrap_or(line);
        check(line)?;
        if line == "settings" {
            return Ok(Self::Settings);
        }
        let (activation, url) = if let Some(rest) = line.strip_prefix("open-activated ") {
            let (token, url) = rest.split_once(' ').ok_or(ControlError::Unknown)?;
            if !is_activation_token(token) {
                return Err(ControlError::Token);
            }
            (Some(token.to_owned()), url)
        } else if let Some(url) = line.strip_prefix("open ") {
            (None, url)
        } else {
            return Err(ControlError::Unknown);
        };
        if url.is_empty() {
            return Err(ControlError::Unknown);
        }
        Ok(Self::Open {
            url: url.to_owned(),
            activation,
        })
    }

    /// The same request without its activation token, if it had one. A
    /// Reroute from before tokens were passed on (0.1.12) refuses
    /// `open-activated`, and understands this one.
    pub fn without_activation(&self) -> Option<Self> {
        match self {
            Self::Open {
                url,
                activation: Some(_),
            } => Some(Self::Open {
                url: url.clone(),
                activation: None,
            }),
            _ => None,
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

    fn open(url: &str, activation: Option<&str>) -> Request {
        Request::Open {
            url: url.into(),
            activation: activation.map(str::to_owned),
        }
    }

    #[test]
    fn every_request_round_trips() {
        for request in [
            open("https://example.com/a?b=c d", None),
            open("https://example.com/a?b=c d", Some("kwin-12")),
            Request::Settings,
        ] {
            let line = request.encode().unwrap();
            assert!(line.ends_with('\n'));
            assert_eq!(Request::decode(&line).unwrap(), request);
        }
    }

    #[test]
    fn a_request_without_a_token_keeps_the_first_format() {
        assert_eq!(
            open("https://a.org/", None).encode().unwrap(),
            "open https://a.org/\n"
        );
        assert_eq!(
            open("https://a.org/", Some("kwin-12")).encode().unwrap(),
            "open-activated kwin-12 https://a.org/\n"
        );
    }

    #[test]
    fn anything_else_is_refused() {
        assert_eq!(Request::decode("open "), Err(ControlError::Unknown));
        assert_eq!(Request::decode("quit"), Err(ControlError::Unknown));
        assert_eq!(Request::decode(""), Err(ControlError::Unknown));
        assert_eq!(
            Request::decode("open-activated kwin-12"),
            Err(ControlError::Unknown)
        );
        assert_eq!(
            Request::decode("open-activated kwin-12 "),
            Err(ControlError::Unknown)
        );
        assert_eq!(
            Request::decode("open https://a.org/\u{0}x"),
            Err(ControlError::ControlCharacters)
        );
        let long = format!("open https://a.org/{}", "x".repeat(MAX_REQUEST));
        assert_eq!(Request::decode(&long), Err(ControlError::TooLong));
    }

    #[test]
    fn only_a_printable_word_is_a_token() {
        assert!(is_activation_token("kwin-12"));
        assert!(is_activation_token(
            "gnome-shell/Firefox/2350-1-host_TIME73"
        ));
        assert!(!is_activation_token(""));
        assert!(!is_activation_token("two words"));
        assert!(!is_activation_token("jeton-été"));
        assert!(!is_activation_token(&"x".repeat(MAX_ACTIVATION_TOKEN + 1)));
        assert_eq!(
            Request::decode("open-activated jeton-été https://a.org/"),
            Err(ControlError::Token)
        );
        assert_eq!(
            open("https://a.org/", Some("two words")).encode(),
            Err(ControlError::Token)
        );
    }

    #[test]
    fn a_url_that_would_break_the_line_is_not_sent() {
        assert_eq!(
            open("https://a.org/\nsettings", None).encode(),
            Err(ControlError::ControlCharacters)
        );
    }

    #[test]
    fn the_longest_url_with_the_longest_token_still_fits() {
        let url = format!("https://a.org/{}", "x".repeat(MAX_URL_LEN - 14));
        let token = "t".repeat(MAX_ACTIVATION_TOKEN);
        assert!(open(&url, Some(&token)).encode().is_ok());
    }

    #[test]
    fn the_token_can_be_left_out() {
        assert_eq!(
            open("https://a.org/", Some("kwin-12")).without_activation(),
            Some(open("https://a.org/", None))
        );
        assert_eq!(open("https://a.org/", None).without_activation(), None);
        assert_eq!(Request::Settings.without_activation(), None);
    }

    proptest! {
        /// Whatever arrives on the socket, decoding never panics.
        #[test]
        fn decoding_is_total(s in "\\PC{0,200}") {
            let _ = Request::decode(&s);
        }

        #[test]
        fn open_round_trips(
            url in "[a-zA-Z0-9:/?#.=&%~_ -]{1,200}",
            token in proptest::option::of("[!-~]{1,64}"),
        ) {
            let request = Request::Open { url, activation: token };
            prop_assert_eq!(Request::decode(&request.encode().unwrap()).unwrap(), request);
        }
    }
}
