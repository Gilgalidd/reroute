# Changelog

All notable changes to Reroute are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

Nothing yet.

## [0.1.4] - 2026-09-22

### Added

- Detection now fills in launch options: a private or incognito window for
  browsers that support one, and one entry per profile for browsers that
  have several. Profile names come from the browser's own configuration
  (`profiles.ini` for the Firefox family, `Local State` for the Chromium
  family). Options you already have keep their id and name, so rules and
  renames survive; nothing is removed.

## [0.1.3] - 2026-09-22

### Added

- A **Check for a new version** button in the settings, under *About*. It
  asks GitHub which release is newest, says whether the running version is
  behind, and offers to open the download page in a browser. Reroute
  downloads and installs nothing by itself, sends nothing about you, and
  makes no request unless the button is pressed.

## [0.1.2] - 2026-09-22

### Changed

- macOS is now a single universal `.dmg` that runs natively on Apple Silicon
  (M1 and later) and on Intel, instead of one download per architecture.

### Fixed

- macOS: the app bundle is now signed ad hoc. Without any signature, Apple
  Silicon Macs reported the downloaded app as "damaged" and refused to open
  it, so only the Intel build could be used there (through Rosetta). The
  release workflow now verifies both architectures and the signature.
- Linux: registering no longer prints a spurious `update-desktop-database`
  failure. The database is refreshed only for the directory Reroute writes
  into, and only when it writes a user-level entry; the packaged entry is
  already registered by the package manager.

## [0.1.1] - 2026-09-18

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

Tagged but never published: superseded by 0.1.1 before release. The tag is
kept so that version comparisons resolve.

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

[Unreleased]: https://github.com/Gilgalidd/reroute/compare/v0.1.4...HEAD
[0.1.4]: https://github.com/Gilgalidd/reroute/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/Gilgalidd/reroute/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/Gilgalidd/reroute/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/Gilgalidd/reroute/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Gilgalidd/reroute/releases/tag/v0.1.0
