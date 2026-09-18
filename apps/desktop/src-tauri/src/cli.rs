//! Command-line handling. The OS hands us the URL as the first positional
//! argument on Linux and Windows; everything else is for humans.

/// What the process was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// Open the settings window (no arguments, or `--settings`).
    Settings,
    /// Route this (not yet validated) URL.
    Pick(String),
    /// Print the version and exit.
    Version,
    /// Register as the default browser from the command line and exit.
    MakeDefault,
}

/// Interpret the arguments after the program name.
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Mode {
    let mut url = None;
    for arg in args {
        match arg.as_str() {
            "--version" | "-V" => return Mode::Version,
            "--settings" => return Mode::Settings,
            "--make-default" => return Mode::MakeDefault,
            "--" => {}
            flag if flag.starts_with('-') && url.is_none() => {
                log::warn!("ignoring unknown option {flag}");
            }
            _ => {
                if url.is_none() {
                    url = Some(arg);
                } else {
                    log::warn!("ignoring extra argument");
                }
            }
        }
    }
    url.map_or(Mode::Settings, Mode::Pick)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_all(args: &[&str]) -> Mode {
        parse(args.iter().map(|s| (*s).to_owned()))
    }

    #[test]
    fn no_arguments_means_settings() {
        assert_eq!(parse_all(&[]), Mode::Settings);
        assert_eq!(parse_all(&["--settings"]), Mode::Settings);
    }

    #[test]
    fn first_positional_is_the_url_even_if_invalid() {
        assert_eq!(
            parse_all(&["https://a.org"]),
            Mode::Pick("https://a.org".into())
        );
        assert_eq!(
            parse_all(&["--", "https://a.org"]),
            Mode::Pick("https://a.org".into())
        );
        assert_eq!(
            parse_all(&["--weird", "https://a.org", "extra"]),
            Mode::Pick("https://a.org".into())
        );
        assert_eq!(
            parse_all(&["javascript:alert(1)"]),
            Mode::Pick("javascript:alert(1)".into())
        );
    }

    #[test]
    fn make_default_is_a_mode() {
        assert_eq!(parse_all(&["--make-default"]), Mode::MakeDefault);
    }

    #[test]
    fn version_wins() {
        assert_eq!(parse_all(&["--version"]), Mode::Version);
        assert_eq!(parse_all(&["https://a.org", "-V"]), Mode::Version);
    }
}
