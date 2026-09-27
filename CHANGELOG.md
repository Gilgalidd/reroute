# Changelog

All notable changes to Reroute are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

Nothing yet.

## [0.1.16] - 2026-09-27

### Fixed

- Windows: the ⚙ button of the picker opened a settings window that never
  responded. Creating a window from a synchronous command deadlocks
  WebView2; the command is now asynchronous.
- Linux: a click reached a Reroute running in the background that had
  stopped responding (a suspended process, for one) only after 23 seconds,
  eleven attempts of two seconds each; it now gives up after about four.

## [0.1.15] - 2026-09-27

### Fixed

- An unreadable `config.toml` left the picker without a single browser, so
  the link could not be opened, whatever the documentation said. The picker
  now offers the browsers installed on the computer meanwhile. Nothing is
  written until *Save*, which keeps the unreadable file as
  `config.toml.broken`.

## [0.1.14] - 2026-09-27

### Fixed

- Windows: *Make Reroute the default* opened File Explorer on the Documents
  folder instead of Settings › Default apps. Explorer took the Settings
  address for a file path; Reroute now opens it through the shell.

## [0.1.13] - 2026-09-27

### Fixed

- Linux, background mode: every browser Reroute opened stayed in the process
  table as a zombie until you logged out, since Firefox and Chromium hand
  the link to their open window and exit. Reroute now waits for them.
- Linux, background mode: the browser received the activation token of the
  click that had started Reroute, long used, instead of its own click's, so
  on Wayland it could open the link without coming to the front. The click
  now hands its token over with the link.
- Linux, background mode: the settings window opened with `reroute` showed
  the configuration as it was at the last link, and saving it could undo an
  edit made to `config.toml` by hand in between.
- AppImage: *Make Reroute the default* pointed the system at the desktop
  entry inside the AppImage, which disappears when it exits. Reroute now
  writes its own entry, which starts the AppImage file.
- AppImage: the browsers Reroute opened inherited the GTK settings the
  AppImage makes for Reroute's own windows, among them the Adwaita theme and
  paths into the AppImage that vanish when it exits.
- *Settings › Rules*: two faulty patterns with the same fault, such as two
  mistyped `domain:` patterns, broke the list of problems. Each problem now
  names its pattern.
- A release whose tag is not a version, such as `nightly`, no longer reads
  as "Up to date (nightly)"; the check reports that it found no version.

- Linux: Reroute took the `APPIMAGE` variable that an application packaged
  as an AppImage passes on when it opens a link for its own, and pointed the
  session's autostart entry at that application. It now counts an AppImage
  as its own only when it runs from inside it.

### Security

- Windows: a link with a quote in it can add arguments to the command the
  system runs; anything after the link is now ignored, options included.
- Linux: release builds ignore `WEBKIT_INSPECTOR_SERVER` and
  `WEBKIT_INSPECTOR_HTTP_SERVER`, which would open WebKit's remote inspector,
  and with it Reroute's settings, on a port every user of the machine can
  reach.
- Linux: a link is only handed to the Reroute running in the background
  through a directory that is the user's alone, should a session share its
  runtime directory by mistake.
- Without a home directory, the configuration goes to a new private
  directory instead of a fixed one in the shared temporary directory, where
  another user could have placed one of their own.
- The picker says which link the choice is for: a link that arrives while
  the user is choosing is no longer opened with the choice made for the
  previous one.
- Releases: the jobs that build the installers can no longer write to the
  repository or the release, and use no cache. Each installer comes with a
  build provenance attestation (`gh attestation verify`). The macOS
  `.app.tar.gz` archive is no longer attached; the `.dmg` holds the same app.

### Changed

- The option to stay in the background shows only on Linux, the one system
  where it does something.

## [0.1.12] - 2026-09-27

### Added

- Linux: Reroute stays in the background and starts with your session, so
  the picker appears about 0.1 s after a click instead of 0.8 s. The
  process the click starts hands the link to the running Reroute over a
  private local socket and exits. *Settings › General* turns it off. After
  installing a newer version, the running Reroute keeps the old one until
  your next login, or until `pkill -x reroute`.
- An update button beside ⚙ in the picker. Reroute checks by itself, once a
  day when it starts, whether a newer version is out, and the button shows
  the answer by its colour: green when up to date, the accent colour when a
  newer version is published, red when the check failed. Pressing it checks
  at once and shows the result in place of the key hint, with a link to the
  download page when there is one. The daily check can be turned off in
  *Settings › General*.

### Changed

- Linux: the picker appears about twice as fast after a click, around 0.8 s
  instead of 1.7 s on a KDE Wayland laptop. WebKitGTK's DMA-BUF renderer
  brought the GPU up for each new window, which took nearly a second; Reroute
  now tells WebKit to draw in shared memory, which a window this small does
  not notice. Set `WEBKIT_DISABLE_DMABUF_RENDERER=0` to go back. The browsers
  Reroute opens do not inherit the setting.

## [0.1.11] - 2026-09-26

### Fixed

- Linux: Reroute's windows showed a generic icon in the task bar and title
  bar on Wayland (KDE, GNOME). The desktop finds a window's icon through the
  entry named after the window's application id, and that id was the binary's
  name, `reroute`, while the packages install `Reroute.desktop`. Reroute now
  names itself `Reroute` at start-up, and an unpackaged binary registers
  `Reroute.desktop` too. Registering removes the `reroute.desktop` entry
  earlier versions wrote at user level, so the menu does not list Reroute
  twice; if you had registered such a build, press *Make Reroute the
  default* once more.

## [0.1.10] - 2026-09-26

### Changed

- Dependencies brought up to date: Tauri 2.12, Vite 8 with its Svelte plugin 7,
  TypeScript 6, toml 1.x, shlex 2, base64 0.23, and for the platform code
  winreg 0.56 (Windows) and icns 0.5 (macOS, now able to read the JPEG 2000
  images of recent application icons). The configuration file Reroute writes
  is unchanged.
- `npm run check` also verifies that the Tauri crate and the Tauri npm
  packages are on the same version, which the Tauri build requires.

## [0.1.9] - 2026-09-26

### Security

- Each window gets only the Tauri permissions it uses. Both had Tauri's
  default core set, which lets page scripts create tray icons and menus,
  query windows or resolve paths; the picker now keeps one permission (to
  listen for its update event) and the settings window none.
- Each release now carries `SHA256SUMS`, the checksum of every installer,
  so a download can be checked. The installers are still not signed with a
  paid certificate.
- The release workflow builds nothing until the full CI has passed on the
  tagged commit, and only the jobs that upload files may write to the
  release.
- GitHub Actions are pinned to commit SHAs instead of tags, which can be
  moved to other code. Dependabot keeps them, the Rust crates and the npm
  packages up to date, proposing each new version only once it is a week
  old. CI no longer runs npm install scripts or keeps its token on disk.

### Changed

- Rust edition 2024, and a pinned Rust toolchain (1.98) so that a new
  compiler release cannot fail the build of unchanged code.
- The documentation now says that Flatpak browsers get no private entry from
  detection (Reroute sees `flatpak`, not the browser) and shows how to add
  one by hand. `SECURITY.md` explains how to report a problem while the
  repository is private.

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
- Settings: a launch option's arguments are typed one per line, like a
  browser's. The field used to split on spaces, so an argument containing
  one, such as `--profile-directory=Profile 1` from the configuration guide,
  could not be entered.
- Picker: *Always use for this domain* no longer fails in silence. When the
  host cannot become a rule (an IPv6 address), the picker says so before
  opening anything. The box is hidden while rules are turned off, since the
  rule it writes would not apply.

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

[Unreleased]: https://github.com/Gilgalidd/reroute/compare/v0.1.16...HEAD
[0.1.16]: https://github.com/Gilgalidd/reroute/compare/v0.1.15...v0.1.16
[0.1.15]: https://github.com/Gilgalidd/reroute/compare/v0.1.14...v0.1.15
[0.1.14]: https://github.com/Gilgalidd/reroute/compare/v0.1.13...v0.1.14
[0.1.13]: https://github.com/Gilgalidd/reroute/compare/v0.1.12...v0.1.13
[0.1.12]: https://github.com/Gilgalidd/reroute/compare/v0.1.11...v0.1.12
[0.1.11]: https://github.com/Gilgalidd/reroute/compare/v0.1.10...v0.1.11
[0.1.10]: https://github.com/Gilgalidd/reroute/compare/v0.1.9...v0.1.10
[0.1.9]: https://github.com/Gilgalidd/reroute/compare/v0.1.8...v0.1.9
[0.1.8]: https://github.com/Gilgalidd/reroute/compare/v0.1.7...v0.1.8
[0.1.7]: https://github.com/Gilgalidd/reroute/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/Gilgalidd/reroute/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/Gilgalidd/reroute/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/Gilgalidd/reroute/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/Gilgalidd/reroute/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/Gilgalidd/reroute/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/Gilgalidd/reroute/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Gilgalidd/reroute/releases/tag/v0.1.0
