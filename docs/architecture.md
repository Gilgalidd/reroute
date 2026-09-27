# Architecture

Reroute is three Rust crates and one small Svelte front end.

```
reroute/
├── crates/
│   ├── core/          reroute-core      pure logic, no OS access, ~all tests live here
│   └── platform/      reroute-platform  discovery, launching, registration, icons, version check, background mode
├── apps/desktop/
│   ├── src-tauri/     reroute           Tauri shell: windows, IPC commands, CLI
│   └── src/           Svelte 5 + TypeScript UI (picker and settings windows)
└── docs/
```

The dependency direction is strict: `core` depends on nothing of ours,
`platform` depends on `core`, the app depends on both. The front end talks to
the app only through fifteen typed IPC commands.

## Life of a click

```
OS opens "reroute https://…"            (macOS: RunEvent::Opened instead)
        │
        ▼
cli::parse ──► resident::hand_over ─── a Reroute runs in the background ──► it
        │                               takes the link and the click's
        │                               activation token (below) ──► exit(0)
        ▼
AppState::load (config.toml; first run: discover browsers)
        │
        ▼
resident::claim ── run_in_background: stay running, picker preloaded
        │
        ▼
incoming::route
   ├─ SafeUrl::parse fails ─────────────► picker shows the refusal
   ├─ rules_enabled && find_match ──────► platform::launch::launch ──► exit(0)
   └─ otherwise ────────────────────────► picker window
                                              │ user picks (or "remember")
                                              ▼
                                        commands::pick ──► launch ──► exit(0)
                                          (in the background: hide the picker)
```

A Reroute running in the background never exits: where the diagram says
`exit(0)`, it hides the picker and waits for the next link. The fast path
(rule matched) never creates a window or initialises the web view, so
rule-based opening costs a few milliseconds. With Reroute running in
the background (Linux), the process the click starts hands its request over
a local socket and exits in well under 0.1 s, and the picker, already loaded,
appears about 0.1 s after the click instead of 0.8 s for a cold start. The
picker window is always created hidden; its page asks to be shown
(`show_picker`) once the new link is on screen.

## `reroute-core`

| Module | Responsibility |
|--------|----------------|
| `url` | `SafeUrl`: the only way to turn a string into something the rest of the program accepts. |
| `browser` | `Browser`, `Launch`, `LaunchPlan` (program + argument vector), `%URL%` substitution, executable checks. |
| `rules` | `Pattern` (`exact`/`domain`/`regex`) with textual form, `Ruleset`, `find_match`. |
| `config` | `Config`/`Settings` (serde + TOML), `validate`, `merge_discovered`, `merge`, `remember_domain`. |
| `store` | `ConfigStore`: atomic, private read/write. |
| `import::hurl` | Converter for Hurl's `UserSettings.json`. |
| `brand` | Executable name → brand key and accent colour for fallback tiles. |
| `private` | The argument that opens a private window, per browser family. |
| `control` | The one-line requests a new Reroute sends the one in the background: `open`, `open-activated` (with the click's activation token), `settings`. |
| `release` | Version comparison, the once-a-day rule, the update status the picker shows. |

Everything here is covered by unit tests and, for the URL and rule engines,
property tests (`proptest`).

## `reroute-platform`

| Module | Linux | Windows | macOS |
|--------|-------|---------|-------|
| `discover` | `.desktop` entries with `x-scheme-handler/http(s)` in XDG data dirs, Flatpak and Snap exports | `HKLM/HKCU\Software\Clients\StartMenuInternet` | `.app` bundles whose `Info.plist` lists `http`/`https` |
| `launch` | `Command` + new process group; a thread waits for the child; the browser gets the click's activation token and none of an AppImage's variables | `Command` + `DETACHED_PROCESS` | same as Linux |
| `register` | user desktop entry + direct edit of `mimeapps.list` (`xdg-settings` best-effort first) | HKCU `ProgId` + `RegisteredApplications`, then opens *Default apps* with `ShellExecuteW` | `LSSetDefaultHandlerForURLScheme` |
| `icons` | file read + magic-byte sniffing | same | same, plus ICNS → PNG |
| `paths` | `directories::ProjectDirs` (+ `REROUTE_CONFIG_DIR` override); cache directory; control socket path | same | same |
| `release` | one HTTPS GET (`ureq`, rustls); the last automatic check kept in the cache directory | same | same |
| `control` | Unix socket and lock file in `$XDG_RUNTIME_DIR/reroute/` | — | — |
| `autostart` | XDG autostart entry | — | — |

Each backend separates a **pure parsing layer** (desktop-entry parser,
registry command-line splitter, plist reader, registry entry list) that is
unit-tested on every CI runner, from a thin enumeration layer that touches
the real system.

## The app (`apps/desktop/src-tauri`)

| File | Responsibility |
|------|----------------|
| `lib.rs` | `run()`: parse CLI, hand over or claim, load state, fast path, build Tauri app, run loop (exit policy, macOS URL events). |
| `cli.rs` | `--settings`, `--make-default`, `--version`, `--background`, first positional = URL. |
| `state.rs` | `AppState`: config (reloaded when edited by hand), pending link and its activation token, errors, update status, background role. |
| `resident.rs` | Linux: staying in the background; hand-over, claim, serving requests. |
| `updates.rs` | The version check: once a day at start-up, and on demand. |
| `incoming.rs` | `route` (rules or picker) and `launch`. |
| `windows.rs` | Picker/settings window creation, focus-loss handling, exit policy. |
| `commands.rs` | IPC commands; each is listed in `build.rs` so Tauri generates a permission for it. |
| `capabilities/*.json` | Which window may call which command. |
| `tauri.conf.json` | CSP, bundling targets, desktop template, icons. |

Windows are created programmatically (none in `tauri.conf.json`) so that the
fast path can skip them entirely.

## The front end (`apps/desktop/src`)

- `App.svelte` routes on the URL hash (`#/picker`, `#/settings`).
- `routes/Picker.svelte` — tiles or a list, keyboard handling
  (`lib/keys.ts`), launch menu, remember checkbox, update button.
- `routes/Settings.svelte` — tabs: Browsers, Rules (with a URL tester),
  General (behaviour, default browser, Hurl import), About. Edits a draft
  copy and saves the whole `Config` in one IPC call; Rust validates.
- `lib/api.ts` — the only file that calls `invoke`; mirrors `commands.rs`.
- `lib/types.ts` — TypeScript mirror of the serialised Rust types.
- Styling: CSS custom properties in `app.css`, light/dark via
  `prefers-color-scheme` or `data-theme`; no inline styles (CSP).

Pure helpers (`keys.ts`, `patterns.ts`, `config.ts`, `lines.ts`, `names.ts`,
`ids.ts`) have Vitest tests. The
two windows have component tests (`routes/*.test.ts`) that render them under
jsdom against a fake IPC bridge (`src/test/ipc.ts`) and drive them like a
user would: pick with a digit key, add a launch option, rename a ruleset,
save, and check the payload sent to Rust.

## Design decisions

- **TOML, not JSON**, for the configuration: comments and hand-editing.
  Hurl's JSON is importable.
- **One process per link, except on Linux.** On Windows and macOS Reroute
  exits after each link: simpler state, no tray icon, and their web views
  start fast enough. WebKitGTK takes about 0.8 s to start, so on Linux
  Reroute stays in the background by default, still without a tray icon. It
  reads `config.toml` again when the file changes, and gives each browser
  the activation token of the click it answers.
- **No icon extraction from executables** (Windows `.exe` resources): it
  would need `unsafe` GDI code for a cosmetic gain. Well-known browsers get a
  brand-coloured tile; any browser can have a custom icon file.
- **Only `http`/`https`**: being the default handler for `mailto:` or
  `ftp:` is a different registration and a different threat model.
