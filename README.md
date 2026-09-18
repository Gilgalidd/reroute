<p align="center">
  <img src="apps/desktop/src/assets/logo.svg" width="112" alt="Reroute logo">
</p>

<h1 align="center">Reroute</h1>

<p align="center"><strong>Choose a browser for each link.</strong></p>

<p align="center">
  <a href="https://github.com/Gilgalidd/reroute/actions/workflows/ci.yml"><img src="https://github.com/Gilgalidd/reroute/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <img src="https://img.shields.io/badge/platforms-Linux%20%7C%20Windows%20%7C%20macOS-blue" alt="Linux, Windows, macOS">
  <img src="https://img.shields.io/badge/built%20with-Rust%20%2B%20Tauri%202-orange" alt="Rust + Tauri 2">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-green" alt="MIT license"></a>
</p>

Reroute registers itself as your default browser. When any application opens
a link, Reroute either applies one of your rules silently or shows a small
picker with the browsers installed on your computer. One click, or one key,
and the link opens where you want it: work profile, private window, the
browser that has the right extensions.

It is a cross-platform re-implementation of the ideas behind
[Hurl](https://github.com/U-C-S/Hurl), written in Rust with a security-first
design and a code base small enough for one person to maintain.

<p align="center">
  <img src="docs/images/picker.png" width="560" alt="The Reroute picker: host, full URL, one tile per browser, a checkbox to remember the choice for the domain">
</p>

## Highlights

- **Works everywhere** — Linux, Windows and macOS from one code base
  (Rust core, Tauri 2 shell, Svelte 5 interface). Native installers for each.
- **Rules that decide for you** — `domain:*.github.com`,
  `regex:^https://open\.spotify\.com/`, `exact:https://example.com/login`.
  First match wins; a rule can target a browser *profile* or a private window.
- **Remember in one click** — tick *Always use for this domain* in the picker
  and a rule is written for you.
- **Keyboard first** — `1`–`9` picks a tile, arrows move, `Enter` confirms,
  `Space` opens the profile menu, `Esc` cancels, `,` opens settings.
- **Finds your browsers** — desktop entries on Linux (including Flatpak and
  Snap), the registry on Windows, application bundles on macOS, with their
  real icons.
- **Zero network, zero shell, zero telemetry** — only `http` and `https`
  links are accepted, browsers are spawned with an argument vector, and the
  configuration file is private to your user. See [the security model](docs/security.md).
- **Plain-text configuration** — one TOML file you can edit by hand, plus an
  importer for Hurl's `UserSettings.json`.

## How it works

```
a link is clicked anywhere ──► the OS starts Reroute with the URL
                                      │
                       URL validated (http/https only, no credentials)
                                      │
                    ┌─── a rule matches ───► browser launched, Reroute exits
                    │
                    └─── no rule ─────────► picker window ► your choice ► launch
```

The rule path never creates a window, so links that match open as fast as
with a plain default browser.

## Install

Download the installer for your platform from the
[Releases](https://github.com/Gilgalidd/reroute/releases) page, install it,
open Reroute and press **Make Reroute the default** in the *General* tab.

| Platform | Package | Becoming the default |
|----------|---------|----------------------|
| Linux    | `.deb`, `.rpm`, `.AppImage` | Immediate: Reroute updates `~/.config/mimeapps.list`. Also `reroute --make-default`. |
| Windows  | `.msi`, NSIS `.exe` | Reroute registers itself and opens *Settings › Default apps*, where you select it (Windows does not let applications set themselves as default). |
| macOS    | `.dmg` | macOS shows its own confirmation dialog. |

Linux needs WebKitGTK 4.1 at runtime (`libwebkit2gtk-4.1-0`); the `.deb` and
`.rpm` packages declare it.

## Use

Click any link outside a browser. The picker appears with your browsers.
Right-click a tile, or press `Space`, for its launch options: a Chrome
profile, a Firefox private window, a kiosk mode, whatever you configured.

Open the settings window from the ⚙ button, by starting Reroute with no
argument, or with `reroute --settings`.

<p align="center">
  <img src="docs/images/settings.png" width="640" alt="The settings window, Rules tab: a ruleset with its browser, launch option and patterns, and a box to try a URL against the saved rules">
</p>

Command-line flags: `--settings`, `--make-default`, `--version`.

## Configure

Everything lives in one TOML file:

| Platform | Path |
|----------|------|
| Linux    | `~/.config/reroute/config.toml` |
| macOS    | `~/Library/Application Support/dev.reroute.desktop/config.toml` |
| Windows  | `%APPDATA%\reroute\reroute\config\config.toml` |

```toml
version = 1

[settings]
rules_enabled = true
close_on_focus_loss = true

[[browsers]]
id = "6d1a4b1e-6a0e-4a5f-9c1b-2f0f5f2f1a11"
name = "Firefox"
path = "/usr/bin/firefox"
args = ["%URL%"]

[[browsers.launches]]
id = "0b9a3c2e-4f33-4a7c-8b0e-1c2d3e4f5a6b"
name = "Private window"
args = ["--private-window", "%URL%"]

[[rulesets]]
name = "Work"
browser = "6d1a4b1e-6a0e-4a5f-9c1b-2f0f5f2f1a11"
launch = "0b9a3c2e-4f33-4a7c-8b0e-1c2d3e4f5a6b"
patterns = ["domain:*.github.com", "regex:^https://open\\.spotify\\.com/"]
```

The full reference, including every pattern kind, is in
[docs/configuration.md](docs/configuration.md).

## Documentation

| Document | What it covers |
|----------|----------------|
| [Configuration reference](docs/configuration.md) | File location, settings, browsers, launches, rulesets, pattern syntax, Hurl import |
| [Security model](docs/security.md) | Threat model, URL validation, process execution, file permissions, web view hardening, supply chain |
| [Architecture](docs/architecture.md) | Crates, modules, life of a click, design decisions |
| [Development guide](docs/development.md) | Toolchain, build, test, manual checklist, releasing |
| [Contributing](CONTRIBUTING.md) · [Changelog](CHANGELOG.md) · [Security policy](SECURITY.md) | Project process |

## Status

Pre-1.0, used daily on Linux; Windows and macOS builds are produced by CI
and need wider testing. The configuration format is versioned (`version = 1`)
and Reroute refuses files written by a newer version rather than guessing.

## License

MIT — see [LICENSE](LICENSE).
