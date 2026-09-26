# Contributing

Thanks for helping. Reroute is meant to stay small enough for one person to
understand in an afternoon, so the bar is "clear and boring" rather than
"clever".

## Ground rules

- **One network call, and only one.** The version check in
  `reroute-platform` is the only reason Reroute may open a socket, and it
  runs only when the user asks. Pull requests adding telemetry, remote
  assets, self-installing updates or a second HTTP client will be declined;
  `deny.toml` enforces the client rule.
- **No shell.** Browsers are spawned with an argument vector, never through
  `sh -c` or `cmd /C`.
- **No `unsafe`** outside `crates/platform/src/register/macos.rs`, and none
  added without a `// SAFETY:` comment and a reviewer.
- **Every behaviour is tested.** Pure logic lives in `crates/core` and must
  come with unit tests; parsers in `crates/platform` must be testable without
  the real OS (feed them strings). Tests never change environment variables:
  they run in parallel threads of one process, so a function that reads
  `PATH` or `XDG_*` takes the value as a parameter and its tests pass their own.
- **Public items are documented.** `missing_docs` is a warning in the
  workspace and CI treats warnings as errors.

## Workflow

1. Read [docs/development.md](docs/development.md) to set up the toolchain.
2. Create a branch from `main`.
3. Run `just check` (or the commands it wraps) before pushing: format, clippy
   with pedantic lints, all Rust and TypeScript tests, `cargo deny`.
4. Open a pull request. Describe *why*; the diff shows *what*.
5. Add a line to `CHANGELOG.md` under "Unreleased".

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/)
(`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`).

## Code style

- Rust: `rustfmt` defaults with `max_width = 100`; clippy pedantic is on.
- TypeScript/Svelte: strict TypeScript, Svelte 5 runes, no inline `style`
  attributes (the CSP forbids them); use `style:` directives instead.
- Prefer a small function with a clear name over a comment explaining a
  block. Comments explain *why*, not *what*.
