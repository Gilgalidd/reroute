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
| Requests on the control socket (Linux, background mode) | The user's own processes | Low | Private directory, checked before any link is sent; one-line protocol with a size cap; the same `SafeUrl` validation |

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
  Windows) with `stdin`/`stdout`/`stderr` closed. A thread waits for it, so
  that a Reroute running in the background leaves no zombie processes.
- The browser gets the user's environment rather than Reroute's own: not the
  variables Reroute sets for its windows, and, when Reroute runs from an
  AppImage, not the ones the AppImage points at its own GTK files (which
  vanish with it) or the theme it forces. Its activation token
  (`XDG_ACTIVATION_TOKEN`, which lets it come to the front on Wayland) is the
  one the desktop gave the click it answers, never one Reroute inherited.
- The system helpers used for default-browser registration on Linux
  (`xdg-settings`, `update-desktop-database`) are located on `PATH` and pass
  the same executable check. They are best-effort only: the authoritative
  step is Reroute's own edit of `~/.config/mimeapps.list`, verified after
  writing. Windows runs no helper: Reroute writes its registry keys itself
  and opens Settings › Default apps through `ShellExecuteW`.

## Configuration file

- Written atomically (temporary file + rename): a crash never leaves a
  truncated configuration.
- Created with mode `0600` in a `0700` directory on Unix. The file names
  programs to run, so other users must not be able to edit it.
- A file that cannot be read is reported, never reset: links still open
  (the picker offers the installed browsers and says the configuration was
  not loaded), the settings window shows the error, and the first save
  moves the file aside as `config.toml.broken` instead of overwriting it.
  Nothing is written until then.
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
- a least-privilege capability per window
  (`apps/desktop/src-tauri/capabilities/`), enforced by Tauri on every call:
  - the picker can read its context, launch the chosen browser, dismiss
    itself, open settings, check for a new version and open the download
    page (both only from its update button), and listen for the two events
    that tell it a new link waits and what the daily version check found. It
    launches a browser only for the link on screen: Rust refuses a choice
    made for a link that has since been replaced;
  - the settings window can read and save the configuration, detect
    browsers, import from Hurl, test a URL, manage the default-browser
    registration and check for a new version. It launches a browser for one
    thing only, opening the download page, whose address is a constant;
  - neither window gets Tauri's `core:default` set, so JavaScript cannot
    create tray icons or menus, query windows or resolve paths;
- no Tauri plugins beyond the core, hence no file-system, shell, HTTP or
  dialog APIs exposed to JavaScript.

Icons are read by Rust, size-capped (2 MiB), type-checked by magic bytes
(PNG, ICO, SVG, ICNS→PNG) and passed to the page as `data:` URLs. SVG is
rendered inside `<img>`, where scripts do not execute.

## Network use

Reroute opens a network connection for one thing only: asking which version
is the newest. It asks

- when Reroute starts, at most once a day, while `check_for_updates` is on
  (the default; *Settings › General* turns it off). The time of the last
  attempt is recorded before asking, so a Reroute that exits quickly does
  not ask again at the next click;
- when you press an update button: the one beside ⚙ in the picker or the
  one in *Settings › About*.

There is no telemetry, no remote icon, and no connection at all on the path
that opens a link: the check runs on a thread of its own.

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

## Staying in the background (Linux)

With `run_in_background` on (the default), the first Reroute keeps running
with its picker loaded, and the session starts it at login through an XDG
autostart entry. A later `reroute <url>` hands its request to it and exits.

- The hand-over uses a Unix socket in `$XDG_RUNTIME_DIR/reroute/`. The
  runtime directory belongs to the user with mode 0700 and Reroute's own
  directory inside it is 0700 too, so only the user's own processes can
  connect. They could already run `reroute <url>` themselves.
- The protocol is one line, `open <url>`, `open-activated <token> <url>` or
  `settings`, capped at the 8 KiB of a URL plus a 256-byte token and refused
  if it holds a control character. The token must be one word of printable
  ASCII. The URL then goes through `SafeUrl` exactly as one from the command
  line would.
- A new Reroute sends a link only into a real directory that belongs to the
  user and that nobody else may open, in case a session shares its runtime
  directory by mistake.
- The running Reroute is the one holding an exclusive lock on a file next to
  the socket. The system drops that lock when the process ends, even in a
  crash, so a leftover socket is never mistaken for a live one.
- A Reroute that gets no answer handles the click itself: a hung background
  process cannot swallow a link.
- The long-running web view only ever shows Reroute's own pages.

## Memory safety and `unsafe`

The workspace denies `unsafe_code`. There are three exceptions, each with
a `SAFETY:` comment:

- `crates/platform/src/register/macos.rs` calls two documented Launch
  Services C functions to read and set the default handler.
- `crates/platform/src/register/windows.rs` calls `ShellExecuteW`, with
  constant strings, to open Settings › Default apps: `explorer.exe` cannot
  open that page's address.
- `prepare_webkit` in `apps/desktop/src-tauri/src/lib.rs` (Linux) sets one
  environment variable and, in release builds, removes two, which Rust marks
  `unsafe` because another thread could read the environment at the same
  time. It runs at the very start of the program, before any other thread
  exists. The variable it sets turns WebKit's GPU start-up off; the browsers
  Reroute starts do not inherit it. The two it removes would open WebKit's
  remote inspector on a TCP port that every user of the machine could reach
  (`WEBKIT_INSPECTOR_SERVER`, `WEBKIT_INSPECTOR_HTTP_SERVER`).

Everything else, including the Windows registry access (`winreg`) and
process spawning, uses safe wrappers.

## Supply chain

- Dependencies are pinned by `Cargo.lock` and `package-lock.json`, and the
  Rust toolchain by `rust-toolchain.toml`.
- CI runs `cargo deny` (licences, bans, sources and the RustSec advisory
  database) and `npm audit` on every push. One advisory is ignored, with its
  reason next to it in `deny.toml`: RUSTSEC-2024-0429, an unsound iterator in
  glib 0.18, which GTK 3 (Tauri's Linux backend) brings in any case and whose
  faulty code Reroute never calls.
- GitHub Actions are pinned to full commit SHAs, never to tags, which their
  owner (or someone who took over their account) can move to other code.
  Checkouts keep no token on disk, and `npm ci --ignore-scripts` runs no
  package's install script.
- Dependabot proposes updates weekly, each version only once it is 7 days
  old, when most hijacked releases have already been found and pulled.
- A release builds nothing until the whole CI passes on the tagged commit.
  Workflows are read-only. The jobs that build the installers run the
  dependencies' code, so they get no token that could change the repository
  or the release, and no cache another workflow could have filled; a last
  job, which runs none of that code, creates the draft release.
- The installers are not signed with a paid certificate. Each release
  carries `SHA256SUMS`, to check that a download is intact, and a build
  provenance attestation, to check that it was built by the release workflow
  from the tagged commit: `gh attestation verify <file> --repo
  Gilgalidd/reroute`.
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
