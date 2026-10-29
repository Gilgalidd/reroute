# Roadmap

What Reroute plans to do next, in no particular order. Nothing here is
promised for a given release; when an item ships, it moves to the
[changelog](CHANGELOG.md). Every item keeps to the
[ground rules](CONTRIBUTING.md#ground-rules): no new network call, no shell,
every behaviour tested.

## Clean links before they open

Many links carry tracking parameters or are wrapped by a mail or chat
service, so a rule sees the wrapper instead of the real destination.

- **Unwrap known redirect wrappers**, such as Microsoft Safe Links
  (`*.safelinks.protection.outlook.com/?url=…`), Teams and Google redirects,
  when the destination is written in the link itself. Rules match the real
  destination.
- **Remove tracking parameters** (`utm_*`, `fbclid`, `gclid` and the like)
  from a list bundled with Reroute, on by default, with an option to keep
  them.
- **Rewrite links** with user-defined find-and-replace patterns
  (`regex:` and replacement), applied before the rules, for instance to send
  `x.com` links to another front end.
- The rule tester in *Settings › Rules* shows each step: the link received,
  the link after cleaning, and the rule that matched.

Out of scope: expanding short links (`bit.ly`, `t.co`), which means asking
the shortener, a network request the ground rules forbid.

## Rules by source application

A rule that applies to every link opened from one program, such as "links
from Teams open in Edge", whatever their address. The picker would show
which program the link came from, and *Always use for this domain* would get
a sibling, *Always use for this application*.

Finding the program is different on each system and has to be confirmed
first:

- Windows: the process that started Reroute, read before the link is handed
  to the running instance.
- macOS: the sender of the Apple event that carries the link.
- Linux: the parent process of `xdg-open`, when there is one; links that
  come through a portal (Flatpak) may not say where they came from.

Where the program cannot be known, such rules do not match and the other
rules apply as before.

## Quicker actions in the picker

- `Ctrl+C` (`⌘C` on macOS), and a button, copy the link and close the picker.
- Edit the link before opening it: a field in place of the URL line, still
  validated as any incoming link (`http` and `https` only, no credentials).
- `Shift` with a click or a number key opens the browser's private entry,
  when it has one, so a private window is reachable without a second tile.
- The key hint at the bottom of the picker lists these keys.

## Translations

The interface in several languages. English and French are there, following
the system language or the *Language* setting, and translations are open to
contributors: each language is one JSON file, which a translator edits from
GitHub's web editor, CI checks it and reports how complete each language is,
and a missing string falls back to English. Messages from Rust come as codes
the page translates. The keys and values of `config.toml` stay in English.
Still to come: the installers in the same languages, and a translation
platform if enough languages arrive. The guide is
[docs/translations.md](docs/translations.md).
