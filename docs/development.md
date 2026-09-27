# Development guide

## Prerequisites

| Tool | Version | Notes |
|------|---------|-------|
| Rust | 1.98 (edition 2024) | Install `rustup`; `rust-toolchain.toml` pins the version and adds clippy and rustfmt, so rustup fetches the right toolchain by itself |
| Node.js | 22 LTS or newer | for the Svelte front end and the Tauri CLI |
| Tauri system deps | — | see below |
| `cargo-deny`, `cargo-llvm-cov` | latest | optional locally, required by CI |
| `just` | optional | one-word shortcuts for the commands below |

### Linux (Debian/Ubuntu)

```bash
sudo apt install build-essential curl wget file pkg-config libssl-dev \
  libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf
```

Fedora: `sudo dnf install webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel librsvg2-devel openssl-devel`.

### Windows

Install the *Desktop development with C++* workload of Visual Studio Build
Tools and the WebView2 runtime (present on Windows 10/11 by default).

### macOS

`xcode-select --install`.

## Building and running

```bash
# once
cd apps/desktop && npm ci && cd ../..

# type-check, lint and test everything (what CI does)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
cd apps/desktop && npm run check && npm test && npm run build

# run the app with hot reload; opens the settings window
cd apps/desktop && npm run tauri dev

# run it on a URL, exactly as the OS would
cd apps/desktop && npm run tauri dev -- -- -- "https://example.com/"

# build installers into target/release/bundle/
cd apps/desktop && npm run tauri build

# a standalone production binary without installers (target/release/reroute)
cargo build --release -p reroute --features custom-protocol
```

`custom-protocol` makes Tauri embed `apps/desktop/dist`; `tauri build`
enables it for you, a bare `cargo build --release` does not and produces a
binary that still loads the Vite dev server.

`just check`, `just dev`, `just dev-url https://…` and `just bundle` wrap the
same commands.

On Linux under Wayland the window icon comes from the desktop entry, which
only exists after registering: run `reroute --make-default` once (it also
installs the icons into `~/.local/share/icons/hicolor`).

Set `RUST_LOG=debug` to see what Reroute decides and why; logs go to stderr.
Set `REROUTE_CONFIG_DIR=/tmp/reroute-dev` to keep a scratch configuration
while developing.

On Linux, mind the background mode. A development build that shares its
configuration with an installed Reroute running in the background hands its
links to that one and exits, and a build that stays in the background points
`~/.config/autostart/Reroute.desktop` at itself. Give development builds
their own `REROUTE_CONFIG_DIR` and `XDG_CONFIG_HOME`, or set
`run_in_background = false` in the scratch configuration. `pkill -x reroute`
stops a Reroute running in the background.

Debug builds honour `WEBKIT_INSPECTOR_HTTP_SERVER=127.0.0.1:9222`, which
serves WebKit's inspector for the picker and settings windows; release
builds ignore it (see [security.md](security.md#memory-safety-and-unsafe)).

Note that the Rust crate embeds `apps/desktop/dist`, so run `npm run build`
(or `tauri dev`, which serves it live) before `cargo test --workspace`.
The pure crates need nothing: `cargo test -p reroute-core -p reroute-platform`.

## Project layout

See [architecture.md](architecture.md). Rule of thumb when adding code:

1. Can it be expressed without touching the OS? Put it in `crates/core` with
   tests.
2. Does it touch files, the registry or processes? Put the parsing in a pure
   function in `crates/platform` (tested), and the I/O next to it.
3. Does it need a window? `apps/desktop/src-tauri`, and add the command to
   `build.rs` and to the right capability file.

## Tests

| Layer | Tool | Run |
|-------|------|-----|
| Rust unit + property tests | `cargo test`, `proptest` | `cargo test --workspace` |
| Cross-platform parsers | same, run on the three OS runners in CI | — |
| Front-end helpers | Vitest | `npm test` in `apps/desktop` |
| Front-end windows | Vitest + jsdom + Testing Library, with a fake IPC bridge (`src/test/ipc.ts`) | `npm test` in `apps/desktop` |
| Type checks | `svelte-check`, `tsc` | `npm run check` |
| Lints | clippy (pedantic, warnings are errors), rustfmt | `cargo clippy --workspace --all-targets -- -D warnings` |
| Supply chain | `cargo deny` (licences, bans, sources, RustSec advisories), `npm audit` | `cargo deny check` |
| Coverage | `cargo llvm-cov` | `cargo llvm-cov --workspace --lcov --output-path lcov.info` |

Proptest writes failing cases to `proptest-regressions/`; commit those files
if you fix a bug they found.

## Manual test checklist before a release

- [ ] Fresh profile (`REROUTE_CONFIG_DIR` empty): first run discovers the
      installed browsers and shows them.
- [ ] `reroute https://example.com/` shows the picker; `1` opens the first
      browser; `Esc` closes without opening.
- [ ] Right-click a tile with launch options; pick one; the right profile opens.
- [ ] Tick "Always use for this domain", open the same link again: no picker.
- [ ] `reroute javascript:alert(1)` shows a refusal, opens nothing.
- [ ] Settings › General › Make default; then click a link in another app.
- [ ] Rules › Try a URL reports the expected rule.
- [ ] Corrupt `config.toml` on purpose: the picker offers the installed
      browsers and shows the error; Settings shows it too, and *Save* keeps
      the file as `config.toml.broken`.
- [ ] Linux: a second link shows the picker at once;
      `~/.config/autostart/Reroute.desktop` exists; turning *Keep Reroute
      running in the background* off removes it, and Reroute exits when its
      windows close.
- [ ] Windows: *Make Reroute the default* opens Settings › Default apps, and
      the picker's ⚙ opens a settings window that responds.

## Releasing

1. Move the "Unreleased" notes of `CHANGELOG.md` under the new version, and
   set that version in the root `Cargo.toml` (then `cargo update -w` for
   `Cargo.lock`), in `apps/desktop/src-tauri/tauri.conf.json`, and with
   `npm version --no-git-tag-version 0.2.0` in `apps/desktop` (which updates
   `package.json` and `package-lock.json`).
2. Tag and push: `git tag -a v0.2.0 -m "Reroute v0.2.0" && git push origin main v0.2.0`.
3. The `release` workflow first runs the whole CI workflow again on the
   tagged commit; if any check fails, nothing is built. It then builds the
   installers on the three platforms, without any right to write, and a
   last job computes `SHA256SUMS`, attests the build provenance of each
   installer and attaches everything to a draft GitHub release. Review,
   then publish.

A tag with a pre-release suffix, such as `v0.2.0-rc.1`, runs the same
workflow and leaves a draft marked as a pre-release: a way to try the
installers before a release. They keep the version of the manifests, since
the MSI format accepts numbers only. Delete the draft and the tag
afterwards.

macOS ships as one universal build (`--target universal-apple-darwin`): the
same `.dmg` runs natively on Apple Silicon and on Intel. The workflow then
checks, on the macOS runner, that the binary contains both `arm64` and
`x86_64` and that the app bundle, both on disk and inside the `.dmg`, has a
valid signature. A failed check fails the release.

### Code signing

- **macOS** apps are signed *ad hoc* (`"signingIdentity": "-"` in
  `tauri.conf.json`). Apple Silicon refuses to run code without any
  signature, and an unsigned bundle downloaded from the web is reported as
  "damaged". Ad-hoc signing needs no certificate; users still confirm the
  first launch because the app is not notarised. To notarise, add the
  `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`,
  `APPLE_PASSWORD` and `APPLE_TEAM_ID` secrets, pass them as environment
  variables to the *Build the installers* step of `release.yml`, and set
  `signingIdentity` to the certificate's name; the Tauri CLI picks them up.
- **Windows** installers are unsigned; SmartScreen asks for confirmation
  until a code-signing certificate is configured.
