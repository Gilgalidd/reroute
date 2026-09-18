//! Editing `mimeapps.list`, the freedesktop file that records default
//! applications per MIME type. Writing it directly is what `xdg-mime` does
//! under the hood on generic desktops, and every desktop environment reads
//! it, so Signpost does not depend on the `xdg-utils` scripts (which on KDE
//! shell out to `qtpaths`, a tool that is not always installed).
//!
//! Spec: <https://specifications.freedesktop.org/mime-apps-spec/latest/>

/// MIME types Signpost claims when made the default browser.
pub const WEB_TYPES: &[&str] = &[
    "x-scheme-handler/http",
    "x-scheme-handler/https",
    "text/html",
];

const DEFAULTS: &str = "[Default Applications]";
const ADDED: &str = "[Added Associations]";

/// Return `text` with `desktop_id` set as default for every [`WEB_TYPES`]
/// entry and listed first in its added associations. Other lines, sections
/// and their order are preserved.
pub fn set_default_browser(text: &str, desktop_id: &str) -> String {
    let mut sections = parse(text);
    {
        let defaults = section_mut(&mut sections, DEFAULTS);
        for mime in WEB_TYPES {
            set_key(defaults, mime, desktop_id);
        }
    }
    {
        let added = section_mut(&mut sections, ADDED);
        for mime in WEB_TYPES {
            let current = get_key(added, mime).unwrap_or_default();
            let mut ids: Vec<&str> = current
                .split(';')
                .filter(|s| !s.is_empty() && *s != desktop_id)
                .collect();
            ids.insert(0, desktop_id);
            let value = format!("{};", ids.join(";"));
            set_key(added, mime, &value);
        }
    }
    render(&sections)
}

/// The desktop ids registered as default for each of [`WEB_TYPES`], in
/// order. `None` when the file has no `[Default Applications]` section;
/// a type without an entry yields an empty string.
pub fn default_web_handlers(text: &str) -> Option<Vec<String>> {
    let sections = parse(text);
    let defaults = sections
        .iter()
        .find(|s| s.header.as_deref() == Some(DEFAULTS))?;
    Some(
        WEB_TYPES
            .iter()
            .map(|mime| {
                get_key(defaults, mime)
                    .and_then(|v| v.split(';').find(|s| !s.is_empty()).map(str::to_owned))
                    .unwrap_or_default()
            })
            .collect(),
    )
}

/// A `[header]` followed by its raw lines (comments and blanks included).
struct Section {
    /// `None` for lines before the first header.
    header: Option<String>,
    lines: Vec<String>,
}

fn parse(text: &str) -> Vec<Section> {
    let mut sections = vec![Section {
        header: None,
        lines: Vec::new(),
    }];
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            sections.push(Section {
                header: Some(trimmed.to_owned()),
                lines: Vec::new(),
            });
        } else if let Some(last) = sections.last_mut() {
            last.lines.push(line.to_owned());
        }
    }
    sections
}

fn section_mut<'a>(sections: &'a mut Vec<Section>, header: &str) -> &'a mut Section {
    let index = sections
        .iter()
        .position(|s| s.header.as_deref() == Some(header))
        .unwrap_or_else(|| {
            sections.push(Section {
                header: Some(header.to_owned()),
                lines: Vec::new(),
            });
            sections.len() - 1
        });
    &mut sections[index]
}

fn get_key(section: &Section, key: &str) -> Option<String> {
    section.lines.iter().find_map(|line| {
        let (k, v) = line.split_once('=')?;
        (k.trim() == key).then(|| v.trim().to_owned())
    })
}

fn set_key(section: &mut Section, key: &str, value: &str) {
    let entry = format!("{key}={value}");
    let existing = section
        .lines
        .iter()
        .position(|line| line.split_once('=').is_some_and(|(k, _)| k.trim() == key));
    if let Some(index) = existing {
        section.lines[index] = entry;
    } else {
        // Insert before trailing blank lines so sections stay tidy.
        let at = section
            .lines
            .iter()
            .rposition(|l| !l.trim().is_empty())
            .map_or(0, |i| i + 1);
        section.lines.insert(at, entry);
    }
}

fn render(sections: &[Section]) -> String {
    let mut out = String::new();
    for (i, section) in sections.iter().enumerate() {
        if let Some(header) = &section.header {
            if i > 0 && !out.is_empty() && !out.ends_with("\n\n") {
                out.push('\n');
            }
            out.push_str(header);
            out.push('\n');
        }
        for line in &section.lines {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXISTING: &str = "[Default Applications]\napplication/pdf=okular.desktop\nx-scheme-handler/http=firefox_firefox.desktop\n\n[Added Associations]\nx-scheme-handler/http=firefox_firefox.desktop;chromium.desktop;\n";

    #[test]
    fn sets_defaults_and_keeps_other_entries() {
        let out = set_default_browser(EXISTING, "signpost.desktop");
        assert!(out.contains("application/pdf=okular.desktop\n"), "{out}");
        assert!(
            out.contains("x-scheme-handler/http=signpost.desktop\n"),
            "{out}"
        );
        assert!(
            out.contains("x-scheme-handler/https=signpost.desktop\n"),
            "{out}"
        );
        assert!(out.contains("text/html=signpost.desktop\n"), "{out}");
        assert!(out.contains("x-scheme-handler/http=signpost.desktop;firefox_firefox.desktop;chromium.desktop;\n"), "{out}");
        assert_eq!(
            default_web_handlers(&out).unwrap(),
            vec!["signpost.desktop"; 3]
        );
        assert_eq!(out.matches("[Default Applications]").count(), 1);
        assert_eq!(out.matches("[Added Associations]").count(), 1);
    }

    #[test]
    fn creates_sections_when_missing() {
        let out = set_default_browser("", "signpost.desktop");
        assert!(out.starts_with("[Default Applications]\n"), "{out}");
        assert!(out.contains("\n[Added Associations]\n"), "{out}");
        assert_eq!(
            default_web_handlers(&out).unwrap(),
            vec!["signpost.desktop"; 3]
        );
        assert_eq!(
            default_web_handlers(EXISTING).unwrap(),
            vec!["firefox_firefox.desktop", "", ""]
        );
        assert_eq!(default_web_handlers(""), None);
        assert_eq!(
            default_web_handlers(
                "[Default Applications]
x-scheme-handler/https=a.desktop;b.desktop
"
            )
            .unwrap(),
            vec!["", "a.desktop", ""]
        );
    }

    #[test]
    fn is_idempotent() {
        let once = set_default_browser(EXISTING, "signpost.desktop");
        let twice = set_default_browser(&once, "signpost.desktop");
        assert_eq!(once, twice);
    }

    #[test]
    fn preserves_comments_and_unknown_sections() {
        let text = "# my file\n[Removed Associations]\ntext/html=old.desktop;\n";
        let out = set_default_browser(text, "signpost.desktop");
        assert!(out.starts_with("# my file\n"), "{out}");
        assert!(
            out.contains("[Removed Associations]\ntext/html=old.desktop;\n"),
            "{out}"
        );
    }
}
