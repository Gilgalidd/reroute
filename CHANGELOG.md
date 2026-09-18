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

- Settings: creating a browser or a launch option failed silently on
  WebKitGTK in dev mode (`crypto.randomUUID` needs a secure context); ids
  now fall back to `getRandomValues`. Unexpected JavaScript errors are shown
  in the settings footer instead of being swallowed.
- Settings: ruleset name, browser and launch selection were not saved
  (element bindings inside the rules loop); they are written explicitly now.

- Linux: desktop entries restricted with `OnlyShowIn`/`NotShowIn` (such as
  the snap Chromium stub whose `Exec` is `/usr/bin/false`) are skipped.
- Linux: registering from an unpackaged binary now installs the application
  icons into `~/.local/share/icons/hicolor`, so the window and task-bar icon
  resolve under Wayland.
- Linux: default-browser registration edits `mimeapps.list` directly and
  verifies it, because `xdg-settings` on KDE needs `qtpaths` and restores
  the previous browser when its own write fails.
