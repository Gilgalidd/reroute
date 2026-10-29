// The pure part of translations: looking a key up, filling its placeholders,
// choosing a plural, picking a language, and the pseudo-language. No DOM and
// no Svelte state, so it can be unit-tested; `i18n.svelte.ts` holds the
// current language. See docs/translations.md.

/** One language's strings, by key. Plural variants end in `#<category>`. */
export type Catalog = Record<string, string>;

/** Values for the `{name}` placeholders of a string. `count` also chooses the plural. */
export type Values = Record<string, string | number>;

/** A piece of a formatted string: plain text, or the value of one placeholder. */
export interface Part {
  text: string;
  /** The placeholder this text replaced, if any. */
  slot?: string;
}

/** The source language, complete by definition, and the fallback of every other. */
export const SOURCE = "en";

/** Generated from the source for tests and layout checks; never chosen automatically. */
export const PSEUDO = "pseudo";

const PLACEHOLDER = /\{([A-Za-z][A-Za-z0-9_]*)\}/g;

const pluralRules = new Map<string, Intl.PluralRules>();

/** The CLDR plural category of `count` in `language` (`one`, `other`…). */
export function pluralCategory(language: string, count: number): Intl.LDMLPluralRule {
  const tag = language === PSEUDO ? SOURCE : language;
  let rules = pluralRules.get(tag);
  if (!rules) {
    rules = new Intl.PluralRules(tag);
    pluralRules.set(tag, rules);
  }
  return rules.select(count);
}

/**
 * The raw string for `key`: from `catalog`, else from `source`. With a
 * numeric `count`, the variant for its plural category, else `#other`.
 * An unknown key comes back as itself, so a mistake shows instead of
 * vanishing.
 */
export function lookup(catalog: Catalog, source: Catalog, language: string, key: string, count?: number): string {
  const candidates =
    typeof count === "number" ? [`${key}#${pluralCategory(language, count)}`, `${key}#other`, key] : [key];
  for (const book of [catalog, source]) {
    for (const candidate of candidates) {
      const text = book[candidate];
      if (text !== undefined) return text;
    }
  }
  return key;
}

/** Split `text` at its placeholders, each replaced by its value. A placeholder without a value stays as written. */
export function split(text: string, values: Values, language: string): Part[] {
  const parts: Part[] = [];
  let last = 0;
  for (const match of text.matchAll(PLACEHOLDER)) {
    const name = match[1] ?? "";
    const value = values[name];
    if (value === undefined) continue;
    if (match.index > last) parts.push({ text: text.slice(last, match.index) });
    parts.push({ text: show(value, language), slot: name });
    last = match.index + match[0].length;
  }
  if (last < text.length) parts.push({ text: text.slice(last) });
  return parts;
}

/** `text` with its placeholders replaced by their values. */
export function format(text: string, values: Values, language: string): string {
  return split(text, values, language)
    .map((part) => part.text)
    .join("");
}

function show(value: string | number, language: string): string {
  if (typeof value === "string") return value;
  return new Intl.NumberFormat(language === PSEUDO ? SOURCE : language).format(value);
}

/** The placeholder names of a string, sorted, for comparing translations. */
export function placeholders(text: string): string[] {
  return [...new Set(Array.from(text.matchAll(PLACEHOLDER), (m) => m[1] ?? ""))].sort();
}

/**
 * The language to show. `preference` is `"auto"` or a tag; `available` the
 * languages Reroute has; `system` the web view's `navigator.languages`, in
 * order of preference. A tag matches exactly (`pt-BR`), then by its base
 * language (`pt`); anything else gives the source language.
 */
export function resolveLanguage(preference: string, available: readonly string[], system: readonly string[]): string {
  if (preference === PSEUDO) return PSEUDO;
  const wanted = preference === "auto" ? system : [preference];
  const byLower = new Map(available.map((tag) => [tag.toLowerCase(), tag]));
  for (const tag of wanted) {
    const lower = tag.toLowerCase();
    const exact = byLower.get(lower);
    if (exact) return exact;
    const base = byLower.get(lower.split("-")[0] ?? "");
    if (base) return base;
  }
  return SOURCE;
}

const ACCENTED: Record<string, string> = Object.fromEntries(
  Array.from("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ").map((c, i) => [
    c,
    Array.from("àƀçđéƒĝĥîĵķłɱñôþǫŕšŧûṽŵẋýžÀƁÇĐÉƑĜĤÎĴĶŁṀÑÔÞǪŔŠŦÛṼŴẊÝŽ")[i] ?? c,
  ]),
);

/** Opens and closes every pseudo-language string; tests look for them. */
export const PSEUDO_OPEN = "[!! ";
export const PSEUDO_CLOSE = " !!]";

/**
 * The pseudo-language form of one string: every ASCII letter accented,
 * a third longer, between markers. Placeholders are left intact. It shows
 * at a glance which texts bypass the catalogue, and which labels a longer
 * language would cut.
 */
export function pseudo(text: string): string {
  let out = "";
  let last = 0;
  for (const match of text.matchAll(PLACEHOLDER)) {
    out += accent(text.slice(last, match.index)) + match[0];
    last = match.index + match[0].length;
  }
  out += accent(text.slice(last));
  return `${PSEUDO_OPEN}${out} ${"~".repeat(Math.max(1, Math.ceil(text.length / 3)))}${PSEUDO_CLOSE}`;
}

function accent(text: string): string {
  return Array.from(text, (c) => ACCENTED[c] ?? c).join("");
}

/** The pseudo-language catalogue, generated from the source one. */
export function pseudoCatalog(source: Catalog): Catalog {
  return Object.fromEntries(Object.entries(source).map(([key, text]) => [key, pseudo(text)]));
}
