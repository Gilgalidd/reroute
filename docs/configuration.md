# Configuration reference

Reroute stores everything in a single TOML file, `config.toml`, created on
first run with the browsers it discovered. The settings window edits this
file; you may also edit it by hand while Reroute is not running.

| Platform | Path |
|----------|------|
| Linux    | `$XDG_CONFIG_HOME/reroute/config.toml` (usually `~/.config/reroute/config.toml`) |
| macOS    | `~/Library/Application Support/dev.reroute.reroute/config.toml` |
| Windows  | `%APPDATA%\reroute\reroute\config\config.toml` |

Set the environment variable `REROUTE_CONFIG_DIR` to an absolute path to use
another directory (handy for a portable install or for bug reports).

Parsing is strict: an unknown key is an error, not a silent no-op. If the file
cannot be read, Reroute still lets you open the link with the browsers it
can find, and shows the error in the picker.

## Top level

```toml
version = 1        # format version; files from a newer Reroute are refused

[settings]         # see "Settings"
[[browsers]]       # see "Browsers"
[[rulesets]]       # see "Rulesets"
```

## Settings

| Key | Default | Meaning |
|-----|---------|---------|
| `rules_enabled` | `true` | Evaluate rulesets before showing the picker. |
| `open_under_cursor` | `false` | Place the picker next to the mouse pointer instead of the screen centre. |
| `close_on_focus_loss` | `true` | Dismiss the picker (opening nothing) when it loses focus. |
| `offer_remember` | `true` | Show the "Always use for this domain" checkbox. |
| `theme` | `"auto"` | `"auto"`, `"light"` or `"dark"`. |

## Browsers

```toml
[[browsers]]
id = "6d1a4b1e-6a0e-4a5f-9c1b-2f0f5f2f1a11"   # stable UUID, referenced by rules
name = "Firefox"                              # shown in the picker
path = "/usr/bin/firefox"                     # absolute path to the executable
args = ["%URL%"]                              # optional; see below
hidden = false                                # optional; hide from the picker
icon = "/usr/share/icons/hicolor/128x128/apps/firefox.png"   # optional

[[browsers.launches]]                         # optional, repeatable
id = "0b9a3c2e-4f33-4a7c-8b0e-1c2d3e4f5a6b"
name = "Private window"
args = ["--private-window", "%URL%"]
```

- `path` must be absolute and point to an existing, executable regular file.
  On macOS use the binary inside the bundle, e.g.
  `/Applications/Firefox.app/Contents/MacOS/firefox` (discovery fills this in).
  On Linux, Flatpak and Snap browsers work through their launcher, e.g.
  `path = "/usr/bin/flatpak"` with
  `args = ["run", "org.mozilla.firefox", "%URL%"]`.
- `args` is an **array**, never a shell string. `%URL%` marks where the link
  goes; it may be a whole argument or part of one (`"--app=%URL%"`). If no
  argument contains it, the URL is appended last.
- `icon` must be a local PNG, SVG or ICO file (ICNS on macOS). Remote icons
  are not supported. Without an icon, the picker draws a coloured tile with
  the browser's initial; well-known browsers get their brand colour.
- `launches` are alternative argument lists shown on right-click. They have
  their own `id` so rules can target a profile directly.

  *Browsers › Detect installed* adds the browser's private window and
  nothing else; profiles are yours to add, as described in
  [Profiles and private windows](#profiles-and-private-windows). Entries you
  already have keep their id and their name, so rules and renames survive a
  new detection; an entry is added only when no existing one has the same
  arguments.

## Profiles and private windows

A launch option is just a different argument list, so anything a browser
accepts on the command line can become an entry in the picker's right-click
menu, and a target for a rule.

### Private windows

Detection adds this one for you. The argument differs by family:

| Browser | Argument |
|---------|----------|
| Firefox, LibreWolf, Waterfox, Floorp, Zen, Tor, Mullvad | `--private-window` |
| Chrome, Chromium, Brave, Vivaldi, Thorium | `--incognito` |
| Edge | `--inprivate` |
| Opera | `--private` |
| GNOME Web | `--incognito-mode` |
| Falkon | `--private-browsing` |

Safari has no such argument, so nothing is offered for it.

### Firefox profiles

Firefox selects a profile either by name or by directory:

```toml
[[browsers.launches]]
name = "Profile: Work"
args = ["-P", "Work", "%URL%"]

[[browsers.launches]]
name = "Profile: Personal"
args = ["--profile", "/home/you/.mozilla/firefox/abc123.personal", "%URL%"]
```

Open `about:profiles` in Firefox to see both: *Name* is what `-P` wants,
*Root Directory* is what `--profile` wants. Prefer `--profile` with the
directory when the profile was created by the profile selector of recent
versions: those profiles are not listed in `profiles.ini`, so `-P` does not
find them.

Snap and Flatpak installations keep their profiles under their own home,
for instance `~/snap/firefox/common/.mozilla/firefox/`; `about:profiles`
shows the right path in every case.

### Chromium profiles

Chromium browsers select a profile by the name of its **directory**, not by
the name you see in the browser:

```toml
[[browsers.launches]]
name = "Profile: Work"
args = ["--profile-directory=Profile 1", "%URL%"]

[[browsers.launches]]
name = "Profile: Personal"
args = ["--profile-directory=Default", "%URL%"]
```

Open `chrome://version` in the profile you want and read *Profile Path*: the
last part of it (`Default`, `Profile 1`, `Profile 2`…) is the value to use.
The same works for Chrome, Edge, Brave, Vivaldi and the rest of the family.

### One thing to expect

If the browser is already running, it usually hands the link to the window
already open and ignores the profile argument. Firefox accepts
`--new-instance` to force a separate one, at the price of an error when that
profile is already in use. This is how the browsers behave on their own;
Reroute passes the arguments and nothing more.

## Rulesets

```toml
[[rulesets]]
name = "Work"                                          # cosmetic
browser = "6d1a4b1e-6a0e-4a5f-9c1b-2f0f5f2f1a11"       # a browser id
launch = "0b9a3c2e-4f33-4a7c-8b0e-1c2d3e4f5a6b"        # optional launch id
patterns = [
  "domain:*.github.com",
  "regex:^https://open\\.spotify\\.com/",
  "exact:https://example.com/login",
]
```

Rulesets are evaluated **top to bottom**, and patterns inside a ruleset in
order; the first pattern that matches decides. Nothing matching means the
picker asks.

### Pattern kinds

| Pattern | Matches when… |
|---------|---------------|
| `exact:<url>` | the whole URL is identical to `<url>`, comparing both the raw text and the normalised form (so `https://x.org` and `https://x.org/` are equivalent). A pattern without a prefix is `exact`. |
| `domain:<host>` | the host is exactly `<host>` (case-insensitive). |
| `domain:*.<host>` | the host is `<host>` **or** any subdomain of it. `*.github.com` matches `github.com` and `docs.github.com` but not `evilgithub.com`. |
| `regex:<expression>` | the expression finds a match anywhere in the normalised URL (lower-case scheme and host). Anchor with `^` when you mean "starts with". Syntax is that of the Rust `regex` crate: no backreferences or look-around, but guaranteed linear-time matching. |

A ruleset that references an unknown browser or launch id makes the whole
file invalid; the settings window prevents this, and hand edits are checked
on load.

## Importing from Hurl

In *Settings › General*, paste the contents of Hurl's `UserSettings.json`.
Browsers and rulesets are converted (`d$` → `domain:`, `s$` → `exact:`,
`r$` → `regex:`), merged with your existing configuration by executable path,
and saved. Anything that cannot be converted (UWP browsers, remote icons,
regexes that only .NET understands) is listed instead of being dropped
silently.
