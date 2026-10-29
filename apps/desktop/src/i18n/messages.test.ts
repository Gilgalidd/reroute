import { describe, expect, it } from "vitest";
import { PSEUDO, format, lookup, placeholders, pluralCategory, pseudo, resolveLanguage, split } from "./messages";

const source = {
  "a.hello": "Hello {name}",
  "a.only": "Only in English",
  "a.files#one": "{count} file",
  "a.files#other": "{count} files",
};
const french = {
  "a.hello": "Bonjour {name}",
  "a.files#one": "{count} fichier",
  "a.files#other": "{count} fichiers",
};

describe("lookup", () => {
  it("prefers the language, then falls back to the source", () => {
    expect(lookup(french, source, "fr", "a.hello")).toBe("Bonjour {name}");
    expect(lookup(french, source, "fr", "a.only")).toBe("Only in English");
  });

  it("returns an unknown key as itself", () => {
    expect(lookup(french, source, "fr", "a.missing")).toBe("a.missing");
  });

  it("chooses the plural variant of the language", () => {
    expect(lookup(source, source, "en", "a.files", 1)).toBe("{count} file");
    expect(lookup(source, source, "en", "a.files", 0)).toBe("{count} files");
    // French counts zero as singular, English does not.
    expect(lookup(french, source, "fr", "a.files", 0)).toBe("{count} fichier");
  });

  it("falls back to #other when a category is missing", () => {
    expect(lookup({ "a.files#other": "{count} pliki" }, source, "pl", "a.files", 3)).toBe("{count} pliki");
  });
});

describe("format and split", () => {
  it("fills placeholders and formats numbers in the language", () => {
    expect(format("Hello {name}", { name: "Ada" }, "en")).toBe("Hello Ada");
    expect(format("{count} files", { count: 1234 }, "en")).toBe("1,234 files");
  });

  it("leaves a placeholder without a value as written", () => {
    expect(format("Hello {name}", {}, "en")).toBe("Hello {name}");
  });

  it("names the parts that came from a placeholder", () => {
    expect(split("Always use for {host}.", { host: "github.com" }, "en")).toEqual([
      { text: "Always use for " },
      { text: "github.com", slot: "host" },
      { text: "." },
    ]);
  });

  it("never interprets a value", () => {
    expect(format("{a}", { a: "{b}" }, "en")).toBe("{b}");
  });
});

describe("placeholders", () => {
  it("lists each name once, sorted", () => {
    expect(placeholders("{b} and {a}, {b}")).toEqual(["a", "b"]);
    expect(placeholders("none")).toEqual([]);
  });
});

describe("resolveLanguage", () => {
  const available = ["en", "fr", "pt-BR"];

  it("follows the system in auto, by exact tag then base language", () => {
    expect(resolveLanguage("auto", available, ["pt-BR", "fr"])).toBe("pt-BR");
    expect(resolveLanguage("auto", available, ["fr-CA"])).toBe("fr");
    expect(resolveLanguage("auto", available, ["de-DE", "fr-FR"])).toBe("fr");
  });

  it("falls back to English", () => {
    expect(resolveLanguage("auto", available, ["de"])).toBe("en");
    expect(resolveLanguage("auto", available, [])).toBe("en");
    expect(resolveLanguage("ja", available, ["fr"])).toBe("en");
  });

  it("takes a chosen language over the system's", () => {
    expect(resolveLanguage("fr", available, ["pt-BR"])).toBe("fr");
  });

  it("gives the pseudo-language only when asked by name", () => {
    expect(resolveLanguage(PSEUDO, available, [])).toBe(PSEUDO);
    expect(resolveLanguage("auto", available, [PSEUDO])).toBe("en");
  });
});

describe("pseudo", () => {
  it("accents every ASCII letter, keeps placeholders, and lengthens the text", () => {
    const text = pseudo("Hello {name}");
    expect(text.startsWith("[!! ")).toBe(true);
    expect(text.endsWith(" !!]")).toBe(true);
    expect(text).toContain("{name}");
    expect(text.replace("{name}", "")).not.toMatch(/[A-Za-z]/);
    expect(text.length).toBeGreaterThan("Hello {name}".length * 1.3);
  });

  it("counts plurals like the source language", () => {
    expect(pluralCategory(PSEUDO, 1)).toBe("one");
    expect(pluralCategory(PSEUDO, 2)).toBe("other");
  });
});
