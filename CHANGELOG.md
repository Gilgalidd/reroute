# Changelog

All notable changes to Reroute are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Component tests for the picker and settings windows (Vitest, jsdom,
  Testing Library) driven through a fake IPC bridge.

### Fixed

- Linux: when the deb/rpm package is installed, registration reuses its
  `Reroute.desktop` entry instead of adding a second user-level entry.
- The `custom-protocol` cargo feature is declared so that direct cargo
  release builds can embed the UI (see the development guide).
- Linux packaging: the desktop-entry template file had kept its old name,
  which made `tauri build` fail. The bundle identifier is now
  `dev.reroute.desktop` (Tauri warns against identifiers ending in `.app`).
- Settings: adding a launch option to a browser that had none failed with
  "undefined is not an object": Rust omits empty collections from the
  configuration it sends, and the UI now fills them in on load.

## [0.1.0] - 2026-09-18

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
- `reroute --make-default` for installers and scripts.

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

[Unreleased]: https://github.com/Gilgalidd/reroute/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Gilgalidd/reroute/releases/tag/v0.1.0
