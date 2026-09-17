//! The rule engine: decide a browser for a URL without asking.
//!
//! Rules are grouped in ordered [`Ruleset`]s; each ruleset owns an ordered
//! list of [`Pattern`]s and names the browser (and optionally the launch) to
//! use. Evaluation is first-match-wins, top to bottom.
//!
//! Patterns are written as `kind:text` strings so that they fit in one line
//! of TOML and one text field in the UI:
//!
//! | Kind     | Example                                 | Matches when …                              |
//! |----------|-----------------------------------------|---------------------------------------------|
//! | `exact`  | `exact:https://example.com/login`       | the whole URL is identical                  |
//! | `domain` | `domain:example.com`                    | the host is exactly `example.com`           |
//! | `domain` | `domain:*.example.com`                  | the host is `example.com` or a subdomain    |
//! | `regex`  | `regex:^https://open\.spotify\.com/`    | the regex finds a match in the URL          |
//!
//! A pattern without a prefix is treated as `exact`, like in Hurl.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::browser::{BrowserId, LaunchId};
use crate::error::PatternError;
use crate::url::SafeUrl;

/// Upper bound on the compiled size of one regex. Keeps memory use and
/// compile time bounded even for hostile patterns. Matching time is always
/// linear thanks to the `regex` crate's guarantees.
const REGEX_SIZE_LIMIT: usize = 1 << 20;

/// One compiled matching pattern. See the [module docs](self) for syntax.
#[derive(Debug, Clone)]
pub enum Pattern {
    /// Whole-URL comparison (against both the raw input and the normalised
    /// form, so `https://x.org` and `https://x.org/` behave the same).
    Exact(String),
    /// Host comparison.
    Domain {
        /// Lower-case host without the `*.` prefix.
        host: String,
        /// Whether subdomains also match.
        subdomains: bool,
    },
    /// Regular expression searched (not anchored) in the normalised URL.
    Regex(regex::Regex),
}

impl Pattern {
    /// Does this pattern match `url`?
    pub fn matches(&self, url: &SafeUrl) -> bool {
        match self {
            Self::Exact(text) => text == url.as_str() || text == url.original(),
            Self::Domain { host, subdomains } => {
                let candidate = url.host();
                candidate == host
                    || (*subdomains
                        && candidate.len() > host.len()
                        && candidate.ends_with(host.as_str())
                        && candidate.as_bytes()[candidate.len() - host.len() - 1] == b'.')
            }
            Self::Regex(re) => re.is_match(url.as_str()),
        }
    }

    /// The `kind` word used in the textual form.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Exact(_) => "exact",
            Self::Domain { .. } => "domain",
            Self::Regex(_) => "regex",
        }
    }
}

impl FromStr for Pattern {
    type Err = PatternError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(PatternError::Empty);
        }
        let (kind, body) = match s.split_once(':') {
            // "https://..." has a colon but is a URL, not a kind prefix.
            Some((k, b)) if is_kind_word(k) && !b.starts_with("//") => (k, b.trim()),
            _ => ("exact", s),
        };
        if body.is_empty() {
            return Err(PatternError::Empty);
        }
        match kind {
            "exact" => Ok(Self::Exact(body.to_owned())),
            "domain" => parse_domain(body),
            "regex" => regex::RegexBuilder::new(body)
                .size_limit(REGEX_SIZE_LIMIT)
                .build()
                .map(Self::Regex)
                .map_err(|e| PatternError::InvalidRegex(e.to_string())),
            other => Err(PatternError::UnknownKind(other.to_owned())),
        }
    }
}

/// A prefix counts as a kind word only if it is a short bare identifier.
/// Unknown identifiers are reported (a typo such as `domian:` must not
/// silently become an exact pattern); URLs are recognised by their `//`.
fn is_kind_word(k: &str) -> bool {
    !k.is_empty() && k.len() <= 16 && k.chars().all(|c| c.is_ascii_alphabetic())
}

fn parse_domain(body: &str) -> Result<Pattern, PatternError> {
    let (host, subdomains) = match body.strip_prefix("*.") {
        Some(rest) => (rest, true),
        None => (body, false),
    };
    let host = host.to_ascii_lowercase();
    let valid = !host.is_empty()
        && !host.starts_with('.')
        && !host.ends_with('.')
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');
    if !valid {
        return Err(PatternError::InvalidDomain(body.to_owned()));
    }
    Ok(Pattern::Domain { host, subdomains })
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact(t) => write!(f, "exact:{t}"),
            Self::Domain {
                host,
                subdomains: true,
            } => write!(f, "domain:*.{host}"),
            Self::Domain {
                host,
                subdomains: false,
            } => write!(f, "domain:{host}"),
            Self::Regex(re) => write!(f, "regex:{}", re.as_str()),
        }
    }
}

impl PartialEq for Pattern {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}
impl Eq for Pattern {}

impl Serialize for Pattern {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Pattern {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// An ordered group of patterns that all lead to the same browser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ruleset {
    /// Label shown in the settings; purely cosmetic.
    #[serde(default)]
    pub name: String,
    /// Browser to open.
    pub browser: BrowserId,
    /// Optional launch of that browser.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch: Option<LaunchId>,
    /// Patterns, evaluated in order.
    #[serde(default)]
    pub patterns: Vec<Pattern>,
}

/// The outcome of a successful rule evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Match {
    /// Index of the winning ruleset in the list that was evaluated.
    pub ruleset: usize,
    /// Textual form of the winning pattern (for display).
    pub pattern: String,
    /// Browser to open.
    pub browser: BrowserId,
    /// Optional launch.
    pub launch: Option<LaunchId>,
}

/// First-match-wins evaluation over `rulesets`.
pub fn find_match(rulesets: &[Ruleset], url: &SafeUrl) -> Option<Match> {
    rulesets.iter().enumerate().find_map(|(index, set)| {
        set.patterns.iter().find(|p| p.matches(url)).map(|p| Match {
            ruleset: index,
            pattern: p.to_string(),
            browser: set.browser,
            launch: set.launch,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn u(s: &str) -> SafeUrl {
        SafeUrl::parse(s).unwrap()
    }
    fn p(s: &str) -> Pattern {
        s.parse().unwrap()
    }

    #[test]
    fn exact_matches_raw_or_normalised() {
        assert!(p("exact:https://x.org").matches(&u("https://x.org")));
        assert!(p("https://x.org/").matches(&u("https://x.org")));
        assert!(!p("https://x.org/a").matches(&u("https://x.org/b")));
    }

    #[test]
    fn domain_exact_and_wildcard() {
        let exact = p("domain:GitHub.com");
        assert!(exact.matches(&u("https://github.com/a")));
        assert!(!exact.matches(&u("https://docs.github.com/")));
        assert!(!exact.matches(&u("https://evilgithub.com/")));

        let wild = p("domain:*.github.com");
        assert!(wild.matches(&u("https://github.com/")));
        assert!(wild.matches(&u("https://docs.github.com/")));
        assert!(wild.matches(&u("https://a.b.github.com/")));
        assert!(!wild.matches(&u("https://evilgithub.com/")));
        assert!(!wild.matches(&u("https://github.com.evil.org/")));
    }

    #[test]
    fn domain_rejects_invalid_hosts() {
        for bad in [
            "domain:",
            "domain:*.",
            "domain:a b",
            "domain:.x",
            "domain:x.",
            "domain:http://x",
        ] {
            assert!(
                matches!(
                    bad.parse::<Pattern>(),
                    Err(PatternError::InvalidDomain(_) | PatternError::Empty)
                ),
                "{bad}"
            );
        }
    }

    #[test]
    fn regex_searches_normalised_url() {
        let re = p(r"regex:^https://open\.spotify\.com/");
        assert!(re.matches(&u("https://OPEN.spotify.com/track/1")));
        assert!(!re.matches(&u("https://spotify.com/")));
        assert!(matches!(
            "regex:(".parse::<Pattern>(),
            Err(PatternError::InvalidRegex(_))
        ));
    }

    #[test]
    fn unknown_kind_and_empty() {
        assert!(matches!(
            "glob:*".parse::<Pattern>(),
            Err(PatternError::UnknownKind(_))
        ));
        assert!(matches!("".parse::<Pattern>(), Err(PatternError::Empty)));
        assert!(matches!(
            "regex:".parse::<Pattern>(),
            Err(PatternError::Empty)
        ));
    }

    #[test]
    fn display_round_trips() {
        for s in [
            "exact:https://x.org/",
            "domain:x.org",
            "domain:*.x.org",
            "regex:^a.b$",
        ] {
            assert_eq!(p(s).to_string(), s);
            assert_eq!(p(&p(s).to_string()), p(s));
        }
        // Bare exact patterns gain their prefix.
        assert_eq!(p("https://x.org/").to_string(), "exact:https://x.org/");
    }

    #[test]
    fn first_match_wins_across_and_within_rulesets() {
        let a = BrowserId::new();
        let b = BrowserId::new();
        let l = LaunchId::new();
        let sets = vec![
            Ruleset {
                name: "A".into(),
                browser: a,
                launch: None,
                patterns: vec![p("domain:a.org"), p("domain:both.org")],
            },
            Ruleset {
                name: "B".into(),
                browser: b,
                launch: Some(l),
                patterns: vec![p("domain:both.org"), p("domain:b.org")],
            },
        ];
        let m = find_match(&sets, &u("https://both.org/")).unwrap();
        assert_eq!((m.ruleset, m.browser, m.launch), (0, a, None));
        assert_eq!(m.pattern, "domain:both.org");
        let m = find_match(&sets, &u("https://b.org/")).unwrap();
        assert_eq!((m.ruleset, m.browser, m.launch), (1, b, Some(l)));
        assert!(find_match(&sets, &u("https://c.org/")).is_none());
        assert!(find_match(&[], &u("https://c.org/")).is_none());
    }

    #[test]
    fn hostile_regex_is_bounded() {
        // Nested repetition would be catastrophic with a backtracking engine.
        let re = p("regex:^(a+)+$");
        let url = u(&format!("https://x.org/{}b", "a".repeat(2000)));
        assert!(!re.matches(&url));
        // Size limit refuses absurd patterns instead of exhausting memory.
        assert!(matches!(
            "regex:(a{1000}){1000}".parse::<Pattern>(),
            Err(PatternError::InvalidRegex(_))
        ));
    }

    proptest! {
        #[test]
        fn wildcard_domain_matches_iff_host_is_self_or_subdomain(
            base in "[a-z]{1,5}\\.[a-z]{2,3}",
            sub in prop::option::of("[a-z]{1,5}"),
            other in "[a-z]{1,5}\\.[a-z]{2,3}",
        ) {
            let pattern: Pattern = format!("domain:*.{base}").parse().unwrap();
            let host = match &sub { Some(s) => format!("{s}.{base}"), None => base.clone() };
            let url = u(&format!("https://{host}/"));
            prop_assert!(pattern.matches(&url));
            let unrelated = u(&format!("https://{other}/"));
            prop_assert_eq!(pattern.matches(&unrelated), other == base);
        }

        #[test]
        fn textual_form_round_trips(text in "(exact:https://[a-z]{1,8}\\.org/|domain:(\\*\\.)?[a-z]{1,8}\\.[a-z]{2,3}|regex:[a-z.^$]{1,10})") {
            let parsed: Pattern = text.parse().unwrap();
            prop_assert_eq!(parsed.to_string(), text);
        }
    }
}
