# Contributing

Thanks for helping. Reroute is meant to stay small enough for one person to
understand in an afternoon, so the bar is "clear and boring" rather than
"clever".

## Ground rules

- **One network call, and only one.** The version check in
  `reroute-platform` is the only reason Reroute may open a network
  connection: when the user presses an update button, and by itself at most
  once a day while `check_for_updates` is on. Pull requests adding
  telemetry, remote assets, self-installing updates or a second HTTP client
  will be declined; `deny.toml` enforces the client rule.
- **No shell.** Browsers are spawned with an argument vector, never through
  `sh -c` or `cmd /C`.
- **No `unsafe`** outside the places listed in
  [docs/security.md](docs/security.md#memory-safety-and-unsafe), and none
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

## Translations

Translators: the guide is [docs/translations.md](docs/translations.md#translating-reroute).
A GitHub account is all it takes; open a
[*New language*](https://github.com/Gilgalidd/reroute/issues/new?template=new-language.yml)
issue first, so that two people do not translate the same language.

Developers: every text the interface shows goes through `t()` and a key in
`apps/desktop/src/i18n/locales/en.json`; `npm test` fails on a text written
in a component, an unused key or a broken placeholder. A message Rust shows
in a window is a code declared with `message_codes!` and a `rust.<code>` key,
never a sentence; `cargo test` prints the list to paste into
`apps/desktop/src/i18n/rust-messages.json`. The rules and the checks are in
[docs/translations.md](docs/translations.md#for-developers).

## Code style

- Rust: `rustfmt` defaults with `max_width = 100`; clippy pedantic is on.
- TypeScript/Svelte: strict TypeScript, Svelte 5 runes, no inline `style`
  attributes (the CSP forbids them); use `style:` directives instead.
- Prefer a small function with a clear name over a comment explaining a
  block. Comments explain *why*, not *what*.
