//! Command-line handling. The OS hands us the URL as the first positional
//! argument on Linux and Windows; everything else is for humans.
//!
//! Nothing after the URL counts, options included. On Windows the system
//! builds the command line from a template, `"reroute.exe" "%1"`, and a link
//! with a quote in it can close the quotes and add arguments of its own.

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
    /// Start in the background with no window (Linux, `--background`): what
    /// the session runs at login so that the picker opens at once later.
    Background,
}

/// Interpret the arguments after the program name.
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Mode {
    let mut options = true;
    for arg in args {
        if options {
            match arg.as_str() {
                "--version" | "-V" => return Mode::Version,
                "--settings" => return Mode::Settings,
                "--make-default" => return Mode::MakeDefault,
                "--background" => return Mode::Background,
                "--" => {
                    options = false;
                    continue;
                }
                flag if flag.starts_with('-') => {
                    log::warn!("ignoring unknown option {flag}");
                    continue;
                }
                _ => {}
            }
        }
        // The URL: the rest is ignored (see the module documentation).
        return Mode::Pick(arg);
    }
    Mode::Settings
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
            parse_all(&["--", "--settings"]),
            Mode::Pick("--settings".into()),
            "after `--`, even an option is the URL"
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
    fn background_is_a_mode() {
        assert_eq!(parse_all(&["--background"]), Mode::Background);
    }

    #[test]
    fn version_is_an_option() {
        assert_eq!(parse_all(&["--version"]), Mode::Version);
        assert_eq!(parse_all(&["-V", "https://a.org"]), Mode::Version);
    }

    /// Windows turns a click on `https://a.org/" --make-default "` into
    /// these arguments, from the template `"reroute.exe" "%1"`.
    #[test]
    fn options_a_link_smuggles_in_after_the_url_are_ignored() {
        for option in ["--make-default", "--settings", "--background", "-V"] {
            assert_eq!(
                parse_all(&["https://a.org/", option, ""]),
                Mode::Pick("https://a.org/".into())
            );
        }
    }
}
