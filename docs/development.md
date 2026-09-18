# Development guide

## Prerequisites

| Tool | Version | Notes |
|------|---------|-------|
| Rust | stable ≥ 1.85 | `rustup` recommended; `rust-toolchain.toml` adds clippy and rustfmt |
| Node.js | 22 LTS or newer | for the Svelte front end and the Tauri CLI |
| Tauri system deps | — | see below |
| `cargo-deny`, `cargo-audit`, `cargo-llvm-cov` | latest | optional locally, required by CI |
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
```

`just check`, `just dev`, `just dev-url https://…` and `just bundle` wrap the
same commands.

On Linux under Wayland the window icon comes from the desktop entry, which
only exists after registering: run `reroute --make-default` once (it also
installs the icons into `~/.local/share/icons/hicolor`).

Set `RUST_LOG=debug` to see what Reroute decides and why; logs go to stderr.
Set `REROUTE_CONFIG_DIR=/tmp/reroute-dev` to keep a scratch configuration
while developing.

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
| Supply chain | `cargo deny`, `cargo audit`, `npm audit` | `cargo deny check`, `cargo audit` |
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
- [ ] Corrupt `config.toml` on purpose: the picker still works and shows the
      error; Settings shows it too.

## Releasing

1. Update `CHANGELOG.md` and the `version` in the root `Cargo.toml`,
   `apps/desktop/package.json` and `apps/desktop/src-tauri/tauri.conf.json`.
2. Tag: `git tag v0.2.0 && git push --tags`.
3. The `release` workflow builds installers on the three platforms and
   attaches them to a draft GitHub release. Review, then publish.

Code signing and notarisation are not configured; installers are unsigned
until certificates are added to the workflow (`.github/workflows/release.yml`).
