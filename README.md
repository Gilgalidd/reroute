# Signpost

**Choose a browser for each link.**

Signpost registers itself as your default browser. When any application opens
a link, Signpost either applies one of your rules silently or shows a small
picker with the browsers installed on your computer. It is a cross-platform
re-implementation of the ideas behind [Hurl](https://github.com/U-C-S/Hurl),
built in Rust with a focus on security and code that one person can maintain.

- **Linux, Windows and macOS** from a single code base (Rust + Tauri 2 + Svelte 5).
- **Rules** (`domain:*.github.com`, `regex:^https://open\.spotify\.com/`,
  `exact:https://example.com/`) send links to a browser, or to a specific
  profile / private window, without asking.
- **Remember with one click**: tick "Always use for this domain" in the picker.
- **Keyboard first**: `1`–`9` picks, arrows move, `Enter` confirms, `Space`
  opens profiles, `Esc` cancels.
- **Zero network, zero shell, zero telemetry.** See [docs/security.md](docs/security.md).
- **Imports Hurl's `UserSettings.json`.**

## Install

Download the installer for your platform from the Releases page
(`.deb`, `.rpm` or `.AppImage`; `.msi` or NSIS `.exe`; `.dmg`), install it,
open Signpost and press **Make Signpost the default** in the General tab.

| Platform | What happens |
|----------|--------------|
| Linux    | `xdg-settings` and `xdg-mime` are updated; done. |
| Windows  | Signpost registers itself and opens *Settings › Default apps* where you pick it (Windows does not allow apps to set themselves as default). |
| macOS    | macOS shows its own confirmation dialog. |

Linux needs WebKitGTK 4.1 at runtime (`libwebkit2gtk-4.1-0`), which the
`.deb`/`.rpm` packages declare as a dependency.

## Use

Click any link outside a browser. The picker appears:

```
┌──────────────────────────────────────────────┐
│ github.com                                   │
│ https://github.com/U-C-S/Hurl                │
│                                              │
│  [1 Firefox ▾]  [2 Chrome ▾]  [3 Brave]      │
│                                              │
│ ☐ Always use for github.com    ⚙   Cancel    │
└──────────────────────────────────────────────┘
```

Right-click a tile (or press `Space`) for its launch options, such as a
Chrome profile or a Firefox private window.

Open the settings window from the ⚙ button, by running `signpost` with no
argument, or with `signpost --settings`.

## Configure

Everything lives in one TOML file that you can also edit by hand:

| Platform | Path |
|----------|------|
| Linux    | `~/.config/signpost/config.toml` |
| macOS    | `~/Library/Application Support/dev.signpost.app/config.toml` |
| Windows  | `%APPDATA%\signpost\signpost\config\config.toml` |

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

The full reference is in [docs/configuration.md](docs/configuration.md).

## Documentation

- [Configuration reference](docs/configuration.md)
- [Security model](docs/security.md)
- [Architecture](docs/architecture.md)
- [Development guide](docs/development.md) — build, test, release
- [Contributing](CONTRIBUTING.md) · [Changelog](CHANGELOG.md) · [Security policy](SECURITY.md)

## Status

Pre-1.0. The configuration format is versioned (`version = 1`) and Signpost
refuses files written by a newer version rather than guessing.

## License

MIT — see [LICENSE](LICENSE).
