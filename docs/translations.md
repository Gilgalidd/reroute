# Translations

Reroute speaks English, French, and any language someone translates. This
page is the guide: the first part is for translators, the rest for
developers.

- [Translating Reroute](#translating-reroute)
- [What is translated, and what is not](#what-is-translated-and-what-is-not)
- [For developers](#for-developers)
- [Releases](#releases)

## Translating Reroute

What you need: a GitHub account. No build, no Rust, no Node: one JSON file
per language, edited in GitHub's web editor, and CI says what is wrong.

### Before you start

Open an issue with the
[*New language*](https://github.com/Gilgalidd/reroute/issues/new?template=new-language.yml)
form, or comment on the open one for your language, so that two people do
not translate the same language at once. To improve a language Reroute
already has, a pull request is enough.

### A new language

1. Open [`apps/desktop/src/i18n/locales/en.json`](../apps/desktop/src/i18n/locales/en.json),
   the reference, and copy its contents.
2. In the same folder, *Add file › Create new file*, named after the
   language's [BCP 47](https://www.rfc-editor.org/info/bcp47) tag:
   `de.json`, `pt-BR.json`, `zh-Hant.json`. A region or a script only when
   the language needs one.
3. Paste, then translate the values, never the keys. Start with
   `language.name`, the language's own name (*Deutsch*, *Português
   (Brasil)*), which the language list shows. Delete the lines you leave
   untranslated: they show in English, and the status (below) lists them as
   missing instead of counting them as done. A partial file is welcome.
4. Add the tag to `CFBundleLocalizations` in
   [`apps/desktop/src-tauri/Info.plist`](../apps/desktop/src-tauri/Info.plist):
   macOS shows Reroute only in the languages it declares.
5. *Commit changes…*, *Create a new branch*, *Propose changes*, then open
   the pull request.
6. Once `desktop.comment` is translated, CI fails until the Linux menus
   describe Reroute in your language too. Its message gives the exact line
   to add to `apps/desktop/src-tauri/reroute.desktop` and to
   `crates/platform/src/register/linux.rs`: add it in the same pull request.

There is no list of languages to update: Reroute picks up every file in
`locales/` by itself.

### An existing language

Edit its file. The *Translations* section of a CI run's summary page lists,
for each language, the share translated and the keys missing;
`npm run i18n:status` prints the same, for those who have the code. A key
that `en.json` no longer has fails CI: delete it.

### Writing a translation

- **Placeholders** such as `{host}` or `{count}` stay exactly as they are,
  but may move: *Toujours utiliser pour {host}*. A translation with other
  placeholders fails CI.
- **Plurals.** A text that depends on a number comes in forms named after
  the [CLDR plural categories](https://www.unicode.org/cldr/charts/latest/supplemental/language_plural_rules.html):
  `browsers.detected#one`, `browsers.detected#other`. Give the forms your
  language uses, `#other` always: French uses `#one` and `#other`, Polish
  `#one`, `#few`, `#many` and `#other`, Japanese `#other` alone. A form your
  language does not have fails CI.
- **Leave as they are** the names *Reroute* and *Hurl*, the file names
  (`config.toml`, `config.toml.broken`, `UserSettings.json`), and what a
  user types: `exact`, `domain`, `regex`, `http`, `https`, `%URL%`.
- **The system's words** for the system's things: *Applications par
  défaut*, as Windows names that page in French, and the name of the
  Settings app in your language.
- **Tone**: like the English, short and plain, no exclamation marks. The
  picker is a small window, and its `picker.*` texts have little room;
  `context.json` says where it is tight.
- **Messages from Rust**, the `rust.*` keys, are what Reroute says when
  something goes wrong. Most follow a colon (*This link was refused:
  {reason}*), so they start with a small letter, as in English. A
  `{detail}` holds what the system or another program said, in English:
  keep it at the end, or after a colon, where an English phrase reads well.
- **Context.** [`context.json`](../apps/desktop/src/i18n/context.json)
  explains the texts that need it: where they show, what a placeholder
  holds.
- **Typography.** Write the non-breaking spaces your language needs as
  escapes, so that reviewers can see them: `\u00a0` before a French colon,
  `\u202f` inside « » and before `;`, `?` and `!`, as `fr.json` does:
  `"Ce lien a été refusé\u00a0: {reason}"`. An invisible character written
  as itself fails CI, and so does a character that changes the direction
  of the text.
- **Only text.** No HTML, no Markdown: a value shows as it is written.
  `desktop.comment` may hold neither `"` nor `\`.

### When CI fails

The failing check names the language and the key. The usual causes:

| The check | What to do |
|-----------|------------|
| *has only keys the source has* | A key `en.json` does not have: a typo in the key, or a text that was removed. Fix the key or delete the line. |
| *keeps the placeholders of the source* | A `{…}` was translated, dropped or added: copy it from `en.json`. |
| *uses the plural categories of its language* | A `#form` your language does not have, or no `#other`. |
| *holds only non-empty plain text* | An empty value, a tag such as `<b>`, or an invisible character. |
| *is a known language tag* | The file is not named after a tag in its usual form: `pt-BR`, not `pt_br`. |
| *names itself* | `language.name` is missing. |
| *declare every language to macOS* | Add the tag to `Info.plist` (step 4 above). |
| *describe Reroute in each language* | Add the line the message gives (step 6 above). |
| *get a description they can hold as it is* | Take `"` or `\` out of `desktop.comment`. |
| Every check fails at once | The file is not valid JSON: a missing comma or quote. |

### Seeing it in Reroute

A merged translation ships with the next release, in *Settings › General ›
Language*, and by itself on systems in that language. To see it before,
build Reroute from the pull request's branch ([development.md](development.md)).

### Review, credit, licence

- **Review.** CI checks the form; the meaning needs a second fluent speaker
  when there is one. A pull request that changes only translation files
  (`locales/`, `context.json`) can be merged on a green CI and a fluent
  reader's approval. One that adds a language also touches `Info.plist` and
  the two desktop entries: its reviewer checks that those changes are the
  tag and the `Comment[…]` line, nothing else. Anything more gets the usual
  review. Machine translation is fine as a first draft if the pull request
  says so and someone fluent has read it.
- **Credit.** Commits keep their author, and the changelog names the
  translator ("German translation by @name").
- **Licence.** Translations are contributed under the MIT licence of the
  project, like any other contribution.

## What is translated, and what is not

| Text | Translated | Where it lives |
|------|------------|----------------|
| Picker and settings windows: labels, buttons, hints, `title` and `aria-label` attributes | Yes | `apps/desktop/src/i18n/locales/<lang>.json` |
| Messages Rust shows in those windows: refused links, configuration and pattern errors, launch failures, Hurl import notes, default-browser instructions | Yes, by the page, from a code Rust sends (see [Messages from Rust](#messages-from-rust)) | same files, keys `rust.*` |
| Settings window title | Yes | same files |
| Desktop entry `Comment`, macOS bundle localisations | Yes, a few lines each | `desktop.comment` in each file, copied to `reroute.desktop` and `register/linux.rs`; the tags in `Info.plist` |
| Installers (NSIS, MSI) | Later (see [Later](#later)) | `tauri.conf.json` |
| Command-line output, logs (`RUST_LOG`) | No: English, so that a bug report reads the same everywhere | — |
| `config.toml` keys and values, pattern syntax | No: they are a file format | — |
| Browser names, ruleset names | No: they are the user's data. Detection writes `Firefox (Private)`, the picker's remembered choices `Remembered: Firefox` and a Hurl import `Ruleset 1`, in English; the user may rename them | — |
| Details from third parties inside a message (an OS error, a regex error, a TOML parse error) | No: passed through as a `{detail}` value | — |
| Documentation | No: English only | — |

## For developers

The design goals, in order:

1. **Keep the ground rules.** No new runtime dependency, no network, no
   change to the Content Security Policy, no `unsafe`, every behaviour
   tested.
2. **Translating needs no toolchain.** One file per language, editable in
   GitHub's web editor; CI says what is wrong.
3. **Developers cannot forget a string.** An untranslatable text, an
   unknown key or a broken placeholder fails a check, not a user.
4. **A translation is data, never code.** A contributor who writes one can
   change what a label says, nothing else (see
   [security.md](security.md#translation-files)).

### Catalogue format

One flat JSON file per language, named after its
[BCP 47](https://www.rfc-editor.org/info/bcp47) tag:

```
apps/desktop/src/i18n/
├── i18n.svelte.ts        t(), tParts(), tRust(), setLanguage(): the current language as $state
├── messages.ts           pure: lookup, placeholders, plurals, language resolution, pseudo-language
├── messages.test.ts      unit tests of messages.ts
├── catalog.test.ts       the checks on the files (below)
├── pseudo.test.ts        both windows drawn in the pseudo-language
├── language.test.ts      choosing the language, in both windows; messages from Rust
├── status.test.ts        tests of scripts/i18n-status.mjs
├── context.json          notes for translators, by key (optional per key)
├── rust-messages.json    the codes Rust sends and their parameters (kept in step by a test)
└── locales/
    ├── en.json           the source: every key, always complete
    └── fr.json
```

```json
{
  "language.name": "Français",
  "picker.cancel": "Annuler",
  "picker.remember": "Toujours utiliser pour {host}",
  "picker.refused": "Ce lien a été refusé\u00a0: {reason}",
  "browsers.detected#one": "{count} navigateur ajouté. Cliquez sur Enregistrer pour le garder.",
  "browsers.detected#other": "{count} navigateurs ajoutés. Cliquez sur Enregistrer pour les garder.",
  "rust.url.scheme_not_allowed": "les liens «\u202f{scheme}\u202f» ne sont pas autorisés, seulement http et https"
}
```

Why this format:

- **JSON is inert.** Vite bundles the files into the page at build time, so
  nothing is fetched at run time and the CSP stays `connect-src` IPC only.
  A TypeScript catalogue would be code running in the web view; a JSON file
  cannot run anything, at build time or later.
- **Flat keys** give one line per string: diffs are readable, merge
  conflicts rare, and every translation platform reads the format.
- **Keys name a place, not a sentence**: `picker.cancel`, not `"Cancel"`. The
  same English word in two places can need two translations.
- **`{name}` placeholders**, nothing more. No HTML, no expressions, no ICU
  syntax, so no parser to write or import.
- **Plurals** are separate keys suffixed with a
  [CLDR plural category](https://cldr.unicode.org/index/cldr-spec/plural-rules)
  (`#zero`, `#one`, `#two`, `#few`, `#many`, `#other`). The page picks one
  with the web view's own `Intl.PluralRules`; each language provides the
  categories it uses, and `#other` is always required.

`language.name` is the language's own name, shown in the language list. It
is the only key every file must have. `desktop.comment` describes Reroute in
the Linux menus; a test says which line to copy where.

### In the code

`t` is the only way a component shows text:

```svelte
<button type="button" onclick={() => api.dismiss()}>{t("picker.cancel")}</button>
<button type="button" title={t("picker.settings.title")} aria-label={t("picker.settings")}>⚙</button>
```

- `t(key, values?)` returns the string of the current language, else the
  English one. Its `key` parameter is typed from `en.json`, so a key that
  does not exist is a `svelte-check` error.
- With a `count` value, `t` picks the plural variant.
- Numbers and dates go through `Intl.NumberFormat` and `Intl.DateTimeFormat`
  with the current language, never string concatenation.
- **No `{@html}`, ever.** Svelte escapes every `{…}`, so a translation, or a
  hostile host name in a placeholder, can only ever be text. Where a string
  needs markup around a value (*Always use for **github.com***), `tParts`
  splits it at the placeholders and the component draws the markup:

  ```svelte
  {#each tParts("picker.remember", { host }) as part}
    {#if part.slot === "host"}<strong>{host}</strong>{:else}{part.text}{/if}
  {/each}
  ```

- Text that is not language at all, such as the product's name or an example
  path in a placeholder (`/usr/bin/firefox`), carries `translate="no"`: it
  tells the pseudo-language check, and the browser's own translation tools,
  to leave it alone.
- Names written into the configuration when the user creates something
  (*New browser*, *New ruleset*) come from `t` too: they are data from then
  on, in the language of the moment.
- The current language lives in a `$state` of `i18n.svelte.ts`, so a change
  in *Settings › General* redraws the window at once. `<html lang>` follows
  it, for screen readers and hyphenation.

### Choosing the language

The setting `language` (*Settings › General › Language*) defaults to
`"auto"`:

| Value | Meaning |
|-------|---------|
| `"auto"` | The first of the web view's `navigator.languages` that Reroute has: the exact tag (`pt-BR`), then its base language (`pt`), else English. |
| a tag, such as `"fr"` | That language, else English if Reroute does not have it. |
| `"pseudo"` | The pseudo-language (below), in development builds; a release takes it for `"auto"`. |

Rust only checks that the value is `auto` or shaped like a language tag
(`ConfigError::InvalidLanguage` otherwise); which languages exist is the
page's business.

The web view already follows the system language, so Rust needs no locale
detection, no new crate and no system call. One exception: on macOS,
WKWebView reports only languages the app bundle declares, so `Info.plist`
lists every shipped language under `CFBundleLocalizations`, and a test
compares that list with `locales/`.

Both windows apply the setting they already receive
(`LaunchContext.settings`, `EditableConfig.config.settings`); a change in
*General* shows at once, and *Discard* brings the saved language back. The
picker's title is the product name, *Reroute*, and stays as it is. The
settings page sets `document.title`, and Rust copies it to the native title
bar (`on_document_title_changed` on that window only), so the page needs no
window permission.

Languages written right to left wait until the style sheet has been checked
with `dir="rtl"`: until then, review does not accept a file for one.

### Messages from Rust

Rust never sends the windows a sentence. It sends what happened, a
`reroute_core::Message`, and the page words it:

```json
{ "code": "url.scheme_not_allowed", "params": { "scheme": "ftp" }, "text": "scheme `ftp` is not allowed (only http and https)" }
```

- Every error a window can show has a `fn message(&self) -> Message`:
  `UrlError`, `PatternError`, `ConfigError`, `LaunchError`, `ImportError`
  and `Error` in `reroute-core`; `PlatformError` and `ReleaseError` in
  `reroute-platform`. It is an exhaustive `match`: a new variant without a
  code does not compile. The notes of a Hurl import or a merge, the
  instructions of `register::Outcome::NeedsUserAction`, and the
  application's own refusals (`pick.new_link`…) are messages too; commands
  fail with a `Message`, never with a `String`.
- Codes are declared with `reroute_core::message_codes!`, one line each, in
  `crates/core/src/message.rs`, `crates/platform/src/message.rs` and
  `apps/desktop/src-tauri/src/messages.rs`:

  ```rust
  URL_SCHEME_NOT_ALLOWED = "url.scheme_not_allowed" (scheme);
  ```

  Each becomes a constant whose `with` takes exactly that many values, so a
  missing or extra value does not compile either.
- A value is text (a name, a path), a number, which the page writes the way
  its language does (`70 000`), or another message: *ruleset “Work”:
  {problem}*, where `problem` is a `pattern.*` message, worded in turn.
- `text` is the English `Display` text. Logs and the command line print it,
  and the page shows it for a code its catalogue does not have.
- The page looks a code up as `rust.<code>` with `tRust(message)`;
  `describeError` does the same for what a failed command throws.
- Rust and the page stay in step through a committed list,
  `apps/desktop/src/i18n/rust-messages.json` (each code and its parameter
  names). A Rust test fails if the list no longer matches the codes, and
  prints the new one to paste; a Vitest test fails if a code in the list has
  no key in `en.json`, if the key has other placeholders, or if a `rust.*`
  key has no code.

Third-party messages (the OS, the `regex` crate, the TOML parser, Tauri)
stay in English inside a `{detail}` parameter: translating them is not ours
to do.

### Checks

All run in `npm test` and `cargo test`, with no new tool.

| Check | Fails when |
|-------|------------|
| Keys | A language has a key `en.json` lacks (a stale translation), or lacks `language.name`. |
| Placeholders | A translation's `{…}` names differ from the English ones. |
| Plurals | A plural key lacks `#other`, or uses a category its language does not have (`Intl.PluralRules(lang).resolvedOptions().pluralCategories`). |
| Content | A value is empty, holds an HTML tag, a control character, or an invisible or bidirectional formatting character (U+200B–U+200F, U+202A–U+202E, U+2066–U+2069), which could make a text read differently from what reviewers saw. |
| No `{@html}` | Any `.svelte` file under `src/` contains `{@html`. |
| Unused keys | A key of `en.json` appears nowhere in the interface's code (keys are always written in full, never built from pieces, so that this check can find them). |
| Nothing left in English | `pseudo.test.ts` renders both windows, in their main states, under a **pseudo-language** generated from `en.json` (`[!! Çàñçéł ~~~ !!]`: accented and a third longer). Any visible text, `title`, `aria-label` or `placeholder` outside the markers, other than the fixtures' data and `translate="no"` elements, was written in a component instead of `en.json`. |
| Rust messages | `rust-messages.json` differs from the codes Rust declares (`cargo test`, which prints the new list), a code in it has no `rust.<code>` key or one with other placeholders, or a `rust.*` key has no code (`npm test`). See [Messages from Rust](#messages-from-rust). |
| `Info.plist` | `CFBundleLocalizations` differs from the files in `locales/`. |
| Desktop entries | `reroute.desktop` or `register/linux.rs` lacks the `Comment[<tag>]` line of a language's `desktop.comment`, or the English one differs from `shortDescription` in `tauri.conf.json`. The failure names the line to add. A `desktop.comment` holding `"` or `\`, which would change the Rust string it is copied into, fails too. |

A **missing** translation is not a failure: the string shows in English.
`npm run i18n:status` (`apps/desktop/scripts/i18n-status.mjs`, plain Node
like `check-tauri-versions.mjs`) prints, for each language, the share of
texts translated, a plural counting once, and the keys missing. CI runs it
after the tests and appends the report to the job's summary page, which
needs no write permission.

In a development build, `language = "pseudo"` in `config.toml` shows the
pseudo-language, to spot by eye a label that a longer language would cut:
French and German run about a third longer than English, and the picker is
a small window.

### Developer rules

1. Every text a user can see goes through `t`; the pseudo-language test
   enforces it.
2. Add the English string to `en.json` only. Other languages fall back to it
   until someone translates it.
3. **Change the meaning, change the key.** A new key makes every
   translation of the old meaning disappear instead of showing an outdated
   one. Delete the old key from every file in the same pull request (the
   key check lists them). Fixing a typo in English keeps the key.
4. Add a line to `context.json` when a string is ambiguous out of context:
   where it shows, how much room it has, what a placeholder holds.
5. A message Rust shows in the UI gets a code (see above), never a sentence
   built with `format!` for the page: declare it, add `rust.<code>` to
   `en.json`, and paste the list the Rust test prints into
   `rust-messages.json`.

## Releases

- The release notes name the languages below 100 % in the *Translations*
  section of the CI run, or in `npm run i18n:status`, as "partly in
  English" ([Releasing](development.md#releasing)). There is no string
  freeze: the English fallback covers what is missing.
- The [manual checklist](development.md#manual-test-checklist-before-a-release)
  opens the picker in French at its minimum size.

## Later

- **Installer languages.** Tauri ships translations of the NSIS and WiX
  installers; `tauri.conf.json` can list Reroute's languages once there are
  a few.
- **A translation platform.** If enough languages arrive, a hosted platform
  such as [Weblate](https://weblate.org/) (free hosting for open-source
  projects) reads this format as it is. It would work only through pull
  requests from its own fork, which go through CI and review like any
  other: it gets no token that can write to this repository.
