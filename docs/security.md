# Security model

Signpost is a default browser that is not a browser: every link clicked
outside a browser passes through it, and it decides which real browser gets
it. That position makes it a target for two things: being tricked into
running something other than a browser, and being used to smuggle a hostile
argument into a browser. This document lists what Signpost trusts, what it
does not, and how it defends itself.

## Inputs and trust

| Input | Source | Trust | Defence |
|-------|--------|-------|---------|
| The URL | Any application, via the OS | **None** | `SafeUrl` validation (below) |
| `config.toml` | The user (and the settings UI) | High, but verified | Strict parsing, referential integrity, path checks, private file mode |
| Installed-browser metadata | `.desktop` files, registry, `Info.plist` | Medium | Only used to *propose* entries; executables are re-checked at launch |
| Hurl import JSON | Pasted by the user | Medium | Same validation as a hand-written config |
| Icon files | Paths from the config | Medium | Size cap, content sniffing, no execution |

## URL validation (`signpost_core::url::SafeUrl`)

Every incoming string must pass **all** of these before it is matched
against rules or handed to a browser:

1. at most 8 KiB (protects command-line limits and regex time);
2. no ASCII control characters (no newlines, no NUL);
3. parses as a URL according to the WHATWG URL standard;
4. scheme is `http` or `https` — `javascript:`, `file:`, `data:`, `vbscript:`,
   custom schemes and `mailto:` are refused;
5. has a non-empty host;
6. has no embedded username or password (`https://user:pw@host/` is a
   classic phishing trick and no browser needs it).

The normalised form (lower-case scheme and host, IDN to punycode) is what
browsers receive. Because it always starts with `http://` or `https://`, the
URL can never be mistaken for a command-line option such as `--profile`.

## Process execution

- Browsers are started with `std::process::Command` (`execve` /
  `CreateProcess`) and an **argument vector**. There is no shell anywhere in
  Signpost, so quoting, `;`, `$(...)` or `%VAR%` have no meaning.
- Before spawning, the executable path must be absolute, exist, be a regular
  file and (on Unix) carry an execute bit. This check runs at launch time, not
  only when the configuration is saved, so a tampered file still cannot point
  at a directory or a non-executable.
- The child is detached (new process group on Unix, `DETACHED_PROCESS` on
  Windows) with `stdin`/`stdout`/`stderr` closed, and Signpost exits.
- System helpers used for default-browser registration (`xdg-settings`,
  `xdg-mime`, `explorer.exe`) are located on `PATH` or `%SystemRoot%` and
  pass the same executable check.

## Configuration file

- Written atomically (temporary file + rename): a crash never leaves a
  truncated configuration.
- Created with mode `0600` in a `0700` directory on Unix. The file names
  programs to run, so other users must not be able to edit it.
- `deny_unknown_fields`: a typo cannot silently disable a rule.
- Newer format versions are refused, not guessed.
- Every rule's `browser`/`launch` reference must resolve; every browser path
  must be absolute; ids must be unique.

## Rule engine

- Regular expressions use the Rust `regex` crate: matching is linear in the
  input, so a hostile pattern or URL cannot hang Signpost (no catastrophic
  backtracking). Compiled size is capped at 1 MiB.
- Domain matching is done on the parsed host with an explicit dot boundary;
  `*.github.com` cannot match `github.com.evil.example`.

## Web view

The UI runs in the platform web view (WebKitGTK, WebView2, WKWebView) with:

- a Content Security Policy of `default-src 'none'` plus `'self'` for
  scripts, styles and fonts, `data:` for icons, and the Tauri IPC origin;
  no inline scripts, no inline styles, no remote content;
- no global Tauri object (`withGlobalTauri: false`);
- a least-privilege capability per window: the picker can only read its
  context, launch, dismiss and open settings; the settings window cannot
  launch a browser at all (`apps/desktop/src-tauri/capabilities/`);
- no Tauri plugins beyond the core, hence no file-system, shell, HTTP or
  dialog APIs exposed to JavaScript.

Icons are read by Rust, size-capped (2 MiB), type-checked by magic bytes
(PNG, ICO, SVG, ICNS→PNG) and passed to the page as `data:` URLs. SVG is
rendered inside `<img>`, where scripts do not execute.

## Zero network

Signpost never opens a socket: no update check, no telemetry, no remote
icons. `deny.toml` bans HTTP client crates from the dependency graph and CI
enforces it with `cargo deny`. Updates come from your package manager or the
Releases page.

## Memory safety and `unsafe`

The workspace denies `unsafe_code`. The single exception is
`crates/platform/src/register/macos.rs`, which calls two documented
Launch Services C functions to read and set the default handler; each call
has a `SAFETY:` comment. Everything else, including the Windows registry
access (`winreg`) and process spawning, uses safe wrappers.

## Supply chain

- Dependencies are pinned by `Cargo.lock` and `package-lock.json`.
- CI runs `cargo deny` (licences, bans, sources, advisories) and
  `cargo audit` / `npm audit` on every push.
- Release builds use `panic = "abort"`, LTO and symbol stripping.

## What Signpost does not protect against

- A compromised user account: anyone who can edit `config.toml` can already
  run programs as you. Signpost does not try to sign its configuration.
- Malicious `.desktop` files or registry entries: they can only *propose*
  a browser entry, which then goes through the same executable checks. The
  same files already control your default browser at the OS level.
- Where the browser goes after receiving the URL. Signpost stops at the
  process boundary.

Please report vulnerabilities as described in [SECURITY.md](../SECURITY.md).
