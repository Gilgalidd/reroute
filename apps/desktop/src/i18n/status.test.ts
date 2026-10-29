// `npm run i18n:status`, the report of how complete each translation is.
import { describe, expect, it } from "vitest";
import { report, status } from "../../scripts/i18n-status.mjs";
import type { Catalog } from "./messages";

const source: Catalog = {
  "language.name": "English",
  "picker.cancel": "Cancel",
  "browsers.detected#one": "Added {count} browser.",
  "browsers.detected#other": "Added {count} browsers.",
};

describe("the translation status", () => {
  it("counts a plural once, through its #other form, and lists what is missing in order", () => {
    const [french] = status(source, { en: source, fr: { "language.name": "Français", "browsers.detected#one": "…" } });
    expect(french).toEqual({
      tag: "fr",
      name: "Français",
      total: 3,
      translated: 1,
      missing: ["picker.cancel", "browsers.detected"],
    });
  });

  it("leaves the source out, sorts by tag, and names a language by its tag if it does not say", () => {
    const languages = status(source, { en: source, fr: source, de: {} });
    expect(languages.map((language) => [language.tag, language.name])).toEqual([
      ["de", "de"],
      ["fr", "English"],
    ]);
  });

  it("gives each share rounded down and the missing keys", () => {
    const text = report(status(source, { fr: { "language.name": "Français", "picker.cancel": "Annuler" } }));
    expect(text).toContain("| Français (`fr`) | 66% (2 of 3) | 1 |");
    expect(text).toContain("Français (`fr`): `browsers.detected`");
  });

  it("says so when every language is complete, or when there is only English", () => {
    expect(report(status(source, { en: source, fr: source }))).toContain("Every language is complete.");
    expect(report(status(source, { en: source }))).toContain("English only.");
  });

  it("writes a language's name as plain text, which cannot break the table", () => {
    const text = report(status(source, { xx: { "language.name": "a|b <img> `c`" } }));
    expect(text).toContain("| a\\|b \\<img\\> \\`c\\` (`xx`) |");
  });
});
