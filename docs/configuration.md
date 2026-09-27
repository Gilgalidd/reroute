# Configuration reference

Reroute stores everything in a single TOML file, `config.toml`, created on
first run with the browsers it discovered. The settings window edits this
file, and you may edit it by hand too. A Reroute running in the background
(Linux) reads it again when it has changed, at the next link or when the
settings open. Keep the settings window closed while you edit by hand: its
*Save* writes the whole file.

| Platform | Path |
|----------|------|
| Linux    | `$XDG_CONFIG_HOME/reroute/config.toml` (usually `~/.config/reroute/config.toml`) |
| macOS    | `~/Library/Application Support/dev.reroute.reroute/config.toml` |
| Windows  | `%APPDATA%\reroute\reroute\config\config.toml` |

Set the environment variable `REROUTE_CONFIG_DIR` to an absolute path to use
another directory (handy for a portable install or for bug reports).

Parsing is strict: an unknown key is an error, not a silent no-op. If the file
cannot be read, links still open: the picker offers the browsers installed
on the computer and shows the error. The settings window shows it too, and
its first *Save* moves the unreadable file aside as `config.toml.broken`, so
the rules it holds can be recovered.

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
| `picker_layout` | `"tiles"` | `"tiles"`, `"list"` (one browser per line, its name in full) or `"two-columns"` (the same list in two columns, read left to right). Also in *Settings › General*. |
| `run_in_background` | `true` | Linux: keep Reroute running with its picker loaded, and start it with the session, so that the picker opens at once. Turned off, Reroute starts for each click and exits after it. *Settings › General* shows this option on Linux only. |
| `check_for_updates` | `true` | Ask once a day, when Reroute starts, whether a newer version is published; the picker's update button shows the answer by its colour. |

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

  Detection creates none of these: a browser's private mode becomes a
  second **browser** entry instead, and profiles are yours to add, both
  described in [Profiles and private windows](#profiles-and-private-windows).

## Profiles and private windows

A launch option is just a different argument list, so anything a browser
accepts on the command line can become an entry in the picker's right-click
menu, and a target for a rule.

### Private windows

*Browsers › Detect installed* handles these: every browser it knows a
private mode for gets a second entry beside it, `Firefox (Private)` next to
`Firefox`, so both are one click away in the picker. The two entries share
the executable and the icon but have their own id, so a rule can send a
domain to the private one. A browser Reroute knows no argument for gets its
ordinary entry and nothing else.

Reroute recognises a browser by the name of the program it starts. A
Flatpak browser starts `/usr/bin/flatpak`, so it is detected, but with no
private entry and no brand colour. Add the private entry by hand: *Add*, the
same executable, and the arguments of the detected entry with the private
argument inserted right after the application id, before `@@u`:

```text
run
--branch=stable
--arch=x86_64
--command=firefox
--file-forwarding
org.mozilla.firefox
--private-window
@@u
%URL%
@@
```

Entries already in the configuration keep their id and their name, and one
is added only when none has the same executable and arguments, so renames
and rules survive a new detection.

The argument differs by family:

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
`r$` → `regex:`) and merged, by executable path, into the settings being
edited; press *Save* to keep them. Anything that cannot be converted (UWP
browsers, remote icons, regexes that only .NET understands) is listed
instead of being dropped silently.
