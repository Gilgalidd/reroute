//! The codes of the application's own messages, and the check that the
//! interface translates every code Rust sends (see docs/translations.md).

reroute_core::message_codes! {
    PICK_NO_LINK = "pick.no_link";
    PICK_NEW_LINK = "pick.new_link";
    PICK_REMEMBER = "pick.remember" (host, problem);
    RELEASE_NO_BROWSER = "release.no_browser";
    RELEASE_NOT_RUN = "release.not_run" (detail);
    WINDOW = "window" (detail);
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    /// The interface's list of the codes it translates: `rust.<code>` in
    /// its catalogue, with these placeholders.
    const LIST: &str = include_str!("../../src/i18n/rust-messages.json");

    /// `codes` as the list is written: one code per line, in order.
    fn listing(codes: &BTreeMap<String, Vec<String>>) -> String {
        let lines: Vec<String> = codes
            .iter()
            .map(|(code, params)| {
                let params: Vec<String> = params.iter().map(|name| format!("\"{name}\"")).collect();
                format!("  \"{code}\": [{}]", params.join(", "))
            })
            .collect();
        format!("{{\n{}\n}}\n", lines.join(",\n"))
    }

    #[test]
    fn the_interface_knows_every_code_rust_sends() {
        let mut codes = BTreeMap::new();
        for (code, params) in [
            reroute_core::message::CODES,
            reroute_platform::message::CODES,
            super::CODES,
        ]
        .concat()
        {
            let mut params: Vec<String> = params.iter().map(ToString::to_string).collect();
            params.sort_unstable();
            let known = codes.insert(code.to_owned(), params);
            assert!(known.is_none(), "`{code}` declared twice");
        }
        let listed: BTreeMap<String, Vec<String>> = serde_json::from_str(LIST).unwrap();
        assert!(
            listed == codes,
            "apps/desktop/src/i18n/rust-messages.json is out of date. Replace it with:\n\n{}\n\
             then give each new code a `rust.<code>` key in en.json.",
            listing(&codes)
        );
        assert_eq!(
            listing(&codes),
            LIST.replace("\r\n", "\n"),
            "the list is in order"
        );
    }
}
