# Changelog

All notable changes to Reroute are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Fixed

- Settings: *Detect installed* and *Import from Hurl* could lose work. Both
  wrote the configuration file straight away, from the last saved state:
  detection threw away edits not saved yet, and an import was erased by the
  next Save, because the window still held the configuration from before it.
  Both now add to the settings being edited, and Save keeps them like any
  other change.
- An unreadable `config.toml` (a typo is enough) could be overwritten, rules
  and all. The settings window showed an empty configuration without saying
  why, and the first Save replaced the file. It now shows the error, and
  saving moves the old file aside as `config.toml.broken` so its rules can be
  recovered.

## [0.1.8] - 2026-09-26

### Added

- The picker can show the browsers as a list instead of tiles, in one column
  or two: *Settings › General › Show browsers as*, or `picker_layout` in the
  configuration file. Each line carries the number key, the icon and the
  full name, so long names such as `Chromium Web Browser (Incognito)` are no
  longer cut. The arrow keys follow the layout. Tiles remain the default.

### Changed

- The "Expressive" look of 0.1.7 is withdrawn: its gradient badges, round
  icons and fixed four-column grid brought small visual defects, so the
  interface is back to the one 0.1.6 shipped. The picker's grid again fits as
  many tiles per row as the window holds, and the arrow keys follow it.

### Fixed

- From the tenth browser on, tiles no longer show a number: only `1`–`9` have
  a key, and a badge that does nothing when pressed was misleading.
- The launch-options chevron on a tile is now a real button beside the tile
  instead of a pretend one nested inside it, which is invalid HTML and was
  unclear to screen readers. It looks and behaves the same.

### Security

- The production Content Security Policy now also sets
  `frame-ancestors 'none'`, as the development one already did: no page may
  embed Reroute's windows in a frame.

## [0.1.7] - 2026-09-23

### Changed

- A new look, "Expressive": a warmer paper-and-ink palette, violet and coral
  accents, rounder tiles, and a three-pixel gradient line along the top of the
  picker. Browser icons are round, the `1`–`9` shortcuts sit in gradient
  badges, a tile lifts under the pointer, and primary buttons carry the
  gradient. The picker's grid is now a fixed four columns of equal tiles. Every
  colour is a variable in one block at the top of `app.css`, light and dark
  alike, and all motion respects `prefers-reduced-motion`. Nothing changed in
  what the application does.

## [0.1.6] - 2026-09-23

### Changed

- A browser's private mode is now a **second entry** beside it, `Firefox
  (Private)` next to `Firefox`, rather than an option behind a right-click.
  Both are one click away in the picker, and a rule can send a domain to
  either. Detection recognises an entry by its executable and its arguments
  together, so running it again adds nothing. The launch option earlier
  versions added stays where it is; remove it if you would rather keep only
  the entry.

## [0.1.5] - 2026-09-23

### Added

- The configuration reference explains how to add profiles and private
  windows by hand, with the arguments each browser family takes and where to
  find a profile's name or directory.

### Changed

- Detection no longer reads a browser's profile list. It still offers a
  private or incognito window, which needs nothing but the browser's name;
  profiles are added by hand. Reading them meant following each browser
  into its own configuration format, and Firefox has since moved its list
  into a database of its own, which is more machinery than this feature is
  worth.

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

[Unreleased]: https://github.com/Gilgalidd/reroute/compare/v0.1.8...HEAD
[0.1.8]: https://github.com/Gilgalidd/reroute/compare/v0.1.7...v0.1.8
[0.1.7]: https://github.com/Gilgalidd/reroute/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/Gilgalidd/reroute/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/Gilgalidd/reroute/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/Gilgalidd/reroute/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/Gilgalidd/reroute/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/Gilgalidd/reroute/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/Gilgalidd/reroute/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Gilgalidd/reroute/releases/tag/v0.1.0
