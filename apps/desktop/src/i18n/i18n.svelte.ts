// The current language and `t`, the only way a component shows text. The
// catalogues are bundled at build time: nothing is fetched while Reroute
// runs. See docs/translations.md.

import type { RustMessage } from "../lib/types";
import en from "./locales/en.json";
import {
  type Catalog,
  type Part,
  type Values,
  PSEUDO,
  SOURCE,
  format,
  lookup,
  pseudoCatalog,
  resolveLanguage,
  split,
} from "./messages";

type BaseKey<K> = K extends `${infer Base}#${string}` ? Base : K;

/** A key of `en.json`, plural variants named without their `#category`. */
export type MessageKey = BaseKey<keyof typeof en>;

const source: Catalog = en;

/** Every file in `locales/`, by language tag: adding a language is adding a file. */
const catalogs: Record<string, Catalog> = Object.fromEntries(
  Object.entries(import.meta.glob<Catalog>("./locales/*.json", { eager: true, import: "default" })).map(
    ([path, catalog]) => [path.slice(path.lastIndexOf("/") + 1, -".json".length), catalog],
  ),
);

let pseudoBook: Catalog | null = null;

function catalogOf(language: string): Catalog {
  if (language === PSEUDO) return (pseudoBook ??= pseudoCatalog(source));
  return catalogs[language] ?? source;
}

/** The languages Reroute has, by tag. */
export const LANGUAGES: readonly string[] = Object.keys(catalogs).sort();

/** A language's own name, as its file gives it (`Français`), for the language list. */
export function languageName(tag: string): string {
  return catalogs[tag]?.["language.name"] ?? tag;
}

function systemLanguages(): readonly string[] {
  return typeof navigator === "undefined" ? [] : navigator.languages;
}

const current = $state({ language: resolveLanguage("auto", LANGUAGES, systemLanguages()) });

/**
 * Show the interface in `preference`, the `language` setting: `"auto"` or
 * a tag. `"pseudo"` works in development builds and tests only; a release
 * takes it for `"auto"`.
 */
export function setLanguage(preference: string): void {
  const wanted = preference === PSEUDO && !import.meta.env.DEV ? "auto" : preference;
  current.language = resolveLanguage(wanted, LANGUAGES, systemLanguages());
  if (typeof document !== "undefined") {
    document.documentElement.lang = current.language === PSEUDO ? SOURCE : current.language;
  }
}

/** The language on screen. */
export function language(): string {
  return current.language;
}

function raw(key: MessageKey, values: Values): string {
  const count = typeof values.count === "number" ? values.count : undefined;
  return lookup(catalogOf(current.language), source, current.language, key, count);
}

/** The text for `key` in the current language, placeholders filled. */
export function t(key: MessageKey, values: Values = {}): string {
  return format(raw(key, values), values, current.language);
}

/**
 * The text for `key` cut at its placeholders, for a component that draws
 * markup around a value (`<strong>`, `<code>`) without `{@html}`.
 */
export function tParts(key: MessageKey, values: Values): Part[] {
  return split(raw(key, values), values, current.language);
}

/**
 * A message from Rust in the current language: the text of `rust.<code>`,
 * its values filled in, a message among them worded in turn. A code the
 * catalogue does not have shows the English text Rust sent with it.
 */
export function tRust(message: RustMessage): string {
  const key = `rust.${message.code}`;
  if (source[key] === undefined) return message.text;
  const values: Values = {};
  for (const [name, value] of Object.entries(message.params)) {
    values[name] = typeof value === "object" ? tRust(value) : value;
  }
  return t(key as MessageKey, values);
}
