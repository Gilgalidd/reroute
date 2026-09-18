# Changelog

All notable changes to Signpost are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Picker window with keyboard navigation (1–9, arrows, Enter, Esc) and a
  per-browser launch menu (profiles, private windows).
- Automatic discovery of installed browsers on Linux (desktop entries),
  Windows (registry) and macOS (application bundles).
- Rule engine with `exact:`, `domain:` and `regex:` patterns, first match wins.
- "Always use for this domain" checkbox that writes a domain rule.
- Settings window: browsers, rules with a URL tester, behaviour, default
  browser registration, import from Hurl.
- Strict URL validation: only `http`/`https`, no credentials, size-limited.
- TOML configuration with atomic, private writes.
- `signpost --make-default` for installers and scripts.

### Fixed

- Linux: desktop entries restricted with `OnlyShowIn`/`NotShowIn` (such as
  the snap Chromium stub whose `Exec` is `/usr/bin/false`) are skipped.
- Linux: default-browser registration edits `mimeapps.list` directly and
  verifies it, because `xdg-settings` on KDE needs `qtpaths` and restores
  the previous browser when its own write fails.
