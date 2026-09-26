# Security model

Reroute is a default browser that is not a browser: every link clicked
outside a browser passes through it, and it decides which real browser gets
it. That position makes it a target for two things: being tricked into
running something other than a browser, and being used to smuggle a hostile
argument into a browser. This document lists what Reroute trusts, what it
does not, and how it defends itself.

## Inputs and trust

| Input | Source | Trust | Defence |
|-------|--------|-------|---------|
| The URL | Any application, via the OS | **None** | `SafeUrl` validation (below) |
| `config.toml` | The user (and the settings UI) | High, but verified | Strict parsing, referential integrity, path checks, private file mode |
| Installed-browser metadata | `.desktop` files, registry, `Info.plist` | Medium | Only used to *propose* entries; executables are re-checked at launch |
| Hurl import JSON | Pasted by the user | Medium | Same validation as a hand-written config |
| Icon files | Paths from the config | Medium | Size cap, content sniffing, no execution |

## URL validation (`reroute_core::url::SafeUrl`)

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
  Reroute, so quoting, `;`, `$(...)` or `%VAR%` have no meaning.
- Before spawning, the executable path must be absolute, exist, be a regular
  file and (on Unix) carry an execute bit. This check runs at launch time, not
  only when the configuration is saved, so a tampered file still cannot point
  at a directory or a non-executable.
- The child is detached (new process group on Unix, `DETACHED_PROCESS` on
  Windows) with `stdin`/`stdout`/`stderr` closed, and Reroute exits.
- System helpers used for default-browser registration (`xdg-settings`,
  `update-desktop-database`, `explorer.exe`) are located on `PATH` or
  `%SystemRoot%` and pass the same executable check. On Linux they are
  best-effort only: the authoritative step is Reroute's own edit of
  `~/.config/mimeapps.list`, verified after writing.

## Configuration file

- Written atomically (temporary file + rename): a crash never leaves a
  truncated configuration.
- Created with mode `0600` in a `0700` directory on Unix. The file names
  programs to run, so other users must not be able to edit it.
- A file that cannot be read is reported, never reset: links still open
  (the picker says the configuration was not loaded), the settings window
  shows the error, and the first save moves the file aside as
  `config.toml.broken` instead of overwriting it.
- `deny_unknown_fields`: a typo cannot silently disable a rule.
- Newer format versions are refused, not guessed.
- Every rule's `browser`/`launch` reference must resolve; every browser path
  must be absolute; ids must be unique.

## Rule engine

- Regular expressions use the Rust `regex` crate: matching is linear in the
  input, so a hostile pattern or URL cannot hang Reroute (no catastrophic
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

## Network use

Reroute opens a socket for one thing only: asking which version is the
newest, and only when you press the button in *Settings › About*. There is
no telemetry, no remote icon, no background traffic, and no connection at
all on the path that opens a link.

That check is deliberately small:

- One HTTPS GET of an address compiled into the binary, with a ten second
  timeout and a size limit on the answer.
- Nothing about you is sent: no version, no configuration, no browser list.
  The request carries a bare `reroute` user agent and nothing else.
- Nothing is downloaded, installed or executed. The answer is compared with
  the running version, and the result is a number and a link.
- The page Reroute offers to open is a constant, never an address taken
  from the answer. A tampered or hostile answer can at worst display a wrong
  version number.
- An answer that does not parse, or that names a version older than or equal
  to the running one, never claims an update is available.

`deny.toml` allows one HTTP client in the dependency graph, only underneath
`reroute-platform`; any other one fails CI. TLS goes through rustls, so
OpenSSL is absent.

## Memory safety and `unsafe`

The workspace denies `unsafe_code`. The single exception is
`crates/platform/src/register/macos.rs`, which calls two documented
Launch Services C functions to read and set the default handler; each call
has a `SAFETY:` comment. Everything else, including the Windows registry
access (`winreg`) and process spawning, uses safe wrappers.

## Supply chain

- Dependencies are pinned by `Cargo.lock` and `package-lock.json`, and the
  Rust toolchain by `rust-toolchain.toml`.
- CI runs `cargo deny` (licences, bans, sources and the RustSec advisory
  database) and `npm audit` on every push.
- GitHub Actions are pinned to full commit SHAs, never to tags, which their
  owner (or someone who took over their account) can move to other code.
  Checkouts keep no token on disk, and `npm ci --ignore-scripts` runs no
  package's install script.
- Dependabot proposes updates weekly, each version only once it is 7 days
  old, when most hijacked releases have already been found and pulled.
- A release builds nothing until the whole CI passes on the tagged commit.
  Workflows are read-only; only the jobs that upload to the draft release
  may write. The installers are not signed with a paid certificate, so each
  release carries `SHA256SUMS` to check a download against.
- Release builds use `panic = "abort"`, LTO and symbol stripping.

## What Reroute does not protect against

- A compromised user account: anyone who can edit `config.toml` can already
  run programs as you. Reroute does not try to sign its configuration.
- Malicious `.desktop` files or registry entries: they can only *propose*
  a browser entry, which then goes through the same executable checks. The
  same files already control your default browser at the OS level.
- Where the browser goes after receiving the URL. Reroute stops at the
  process boundary.

Please report vulnerabilities as described in [SECURITY.md](../SECURITY.md).
