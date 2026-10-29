// Checks on the translation files themselves; see docs/translations.md. A
// missing translation is not an error (the text shows in English), but
// anything that would show a wrong or misleading text is.
import { describe, expect, it } from "vitest";
import linuxRegister from "../../../../crates/platform/src/register/linux.rs?raw";
import plist from "../../src-tauri/Info.plist?raw";
import desktopEntry from "../../src-tauri/reroute.desktop?raw";
import tauriConf from "../../src-tauri/tauri.conf.json";
import context from "./context.json";
import { type Catalog, SOURCE, placeholders } from "./messages";
import rustMessages from "./rust-messages.json";

// Read through Vite, like the application reads them.
const locales: Record<string, Catalog> = Object.fromEntries(
  Object.entries(import.meta.glob<Catalog>("./locales/*.json", { eager: true, import: "default" })).map(
    ([path, catalog]) => [path.slice(path.lastIndexOf("/") + 1, -".json".length), catalog],
  ),
);
const source: Catalog = locales[SOURCE] ?? {};

/** Every source file of the interface, tests excepted: its path and its text. */
const files = Object.entries(
  import.meta.glob<string>(["../**/*.svelte", "../**/*.ts", "!../**/*.test.ts"], {
    eager: true,
    query: "?raw",
    import: "default",
  }),
).map(([path, text]) => ({ path, text }));

/** `key` without its plural `#category`. */
function base(key: string): string {
  return key.split("#")[0] ?? key;
}

// Bidirectional formatting and invisible characters can make a text read
// differently from what its reviewers saw; control characters have no place
// in a label.
const FORBIDDEN = /[\u0000-\u001f\u007f-\u009f\u200b-\u200f\u202a-\u202e\u2060-\u2064\u2066-\u2069\ufeff]/;
const TAG = /<\/?[A-Za-z]/;
const CATEGORIES = new Set(["zero", "one", "two", "few", "many", "other"]);

describe("translation files", () => {
  it("include the source language", () => {
    expect(Object.keys(source).length).toBeGreaterThan(0);
  });

  for (const [language, catalog] of Object.entries(locales)) {
    describe(language, () => {
      it("is a known language tag", () => {
        expect(() => new Intl.PluralRules(language)).not.toThrow();
        expect(Intl.getCanonicalLocales(language)).toEqual([language]);
      });

      it("names itself", () => {
        expect(catalog["language.name"]).toBeTruthy();
      });

      it("has only keys the source has", () => {
        const plurals = new Set(Object.keys(source).filter((k) => k.includes("#")).map(base));
        const stale = Object.keys(catalog).filter((key) =>
          key.includes("#") ? !plurals.has(base(key)) : source[key] === undefined,
        );
        expect(stale).toEqual([]);
      });

      it("holds only non-empty plain text", () => {
        const bad = Object.entries(catalog).filter(
          ([, text]) => typeof text !== "string" || text.trim() === "" || FORBIDDEN.test(text) || TAG.test(text),
        );
        expect(bad).toEqual([]);
      });

      it("keeps the placeholders of the source", () => {
        const wrong = Object.entries(catalog).filter(([key, text]) => {
          const english = source[key] ?? source[`${base(key)}#other`];
          return english !== undefined && placeholders(text).join() !== placeholders(english).join();
        });
        expect(wrong).toEqual([]);
      });

      it("uses the plural categories of its language, #other always", () => {
        const allowed = new Set(new Intl.PluralRules(language).resolvedOptions().pluralCategories);
        const wrong = Object.keys(catalog)
          .filter((key) => key.includes("#"))
          .filter((key) => {
            const category = key.slice(key.indexOf("#") + 1);
            return !CATEGORIES.has(category) || !allowed.has(category as Intl.LDMLPluralRule);
          });
        expect(wrong).toEqual([]);
        const bases = new Set(Object.keys(catalog).filter((k) => k.includes("#")).map(base));
        const withoutOther = [...bases].filter((key) => catalog[`${key}#other`] === undefined);
        expect(withoutOther).toEqual([]);
      });
    });
  }
});

describe("the source catalogue", () => {
  const code = files.map((file) => file.text).join("\n");

  it("is checked against the interface's own files", () => {
    expect(files.some((file) => file.path.endsWith("routes/Picker.svelte"))).toBe(true);
  });

  it("has no key that the interface does not use", () => {
    // `language.name` is read from each file to list the languages,
    // `desktop.comment` goes to the system's files (below), and the
    // `rust.*` keys follow the codes Rust sends (below too).
    const unused = [...new Set(Object.keys(source).map(base))].filter(
      (key) =>
        key !== "language.name" &&
        key !== "desktop.comment" &&
        !key.startsWith("rust.") &&
        !code.includes(`"${key}"`),
    );
    expect(unused).toEqual([]);
  });

  it("has notes only for keys that exist", () => {
    const keys = new Set(Object.keys(source).map(base));
    expect(Object.keys(context).filter((key) => !keys.has(key))).toEqual([]);
  });
});

// The codes Rust sends, with their parameters: a Rust test keeps this list
// in step with the code.
describe("messages from Rust", () => {
  const codes: Record<string, string[]> = rustMessages;
  const keys = Object.keys(source).filter((key) => key.startsWith("rust."));

  it("each have a key with their parameters as placeholders", () => {
    const wrong = Object.entries(codes).filter(([code, params]) => {
      const english = source[`rust.${code}`];
      return english === undefined || placeholders(english).join() !== [...params].sort().join();
    });
    expect(wrong.map(([code]) => code)).toEqual([]);
  });

  it("are the only ones under rust.*", () => {
    expect(keys.filter((key) => codes[key.slice("rust.".length)] === undefined)).toEqual([]);
  });
});

describe("the system's files", () => {
  const languages = Object.keys(locales).sort();
  const others = languages.filter((language) => language !== SOURCE);

  it("declare every language to macOS, which tells the web view no other", () => {
    const array = /<key>CFBundleLocalizations<\/key>\s*<array>([\s\S]*?)<\/array>/.exec(plist)?.[1] ?? "";
    const declared = Array.from(array.matchAll(/<string>([^<]*)<\/string>/g), (m) => m[1]).sort();
    expect(declared).toEqual(languages);
  });

  it("get a description they can hold as it is", () => {
    // `desktop.comment` is copied into a Rust string in register/linux.rs,
    // where a backslash sequence would start a new line of the desktop
    // entry (an `Exec=` line, say) and a double quote would end the string.
    const backslash = String.fromCharCode(92);
    const unsafe = languages.filter((language) => {
      const comment = locales[language]?.["desktop.comment"] ?? "";
      return comment.includes('"') || comment.includes(backslash);
    });
    expect(unsafe, "desktop.comment may hold neither a double quote nor a backslash").toEqual([]);
  });

  it("describe Reroute in each language in the Linux menus", () => {
    const english = source["desktop.comment"];
    expect(tauriConf.bundle.shortDescription).toBe(english);
    expect(linuxRegister).toContain(`\\nComment=${english}\\n`);
    const lines = desktopEntry.split(/\r?\n/);
    for (const language of others) {
      const comment = locales[language]?.["desktop.comment"];
      if (!comment) continue;
      const line = `Comment[${language.replace("-", "_")}]=${comment}`;
      expect(lines, `apps/desktop/src-tauri/reroute.desktop needs the line ${line}`).toContain(line);
      expect(linuxRegister, `crates/platform/src/register/linux.rs needs ${line}`).toContain(`\\n${line}\\n`);
    }
  });
});

/**
 * A character a reviewer cannot see: a non-breaking space, a zero-width or
 * bidirectional formatting character, a byte order mark.
 */
function invisible(code: number): boolean {
  return (
    code === 0xa0 ||
    (code >= 0x200b && code <= 0x200f) ||
    (code >= 0x202a && code <= 0x202f) ||
    (code >= 0x2060 && code <= 0x2064) ||
    (code >= 0x2066 && code <= 0x2069) ||
    code === 0xfeff
  );
}

describe("the sources", () => {
  // What reviewers read, tests, translations and documentation included,
  // where such a character is written as its escape sequence so that a
  // diff shows it.
  const sources = import.meta.glob<string>(
    [
      "../**/*.{ts,svelte,json}",
      "../../scripts/*",
      "../../src-tauri/src/**/*.rs",
      "../../../../crates/*/src/**/*.rs",
      "../../../../*.md",
      "../../../../docs/*.md",
    ],
    { eager: true, query: "?raw", import: "default" },
  );

  it("are all read", () => {
    const paths = Object.keys(sources);
    for (const end of ["locales/fr.json", "i18n-status.mjs", "commands.rs", "message.rs", "translations.md", "CHANGELOG.md"]) {
      expect(paths.some((path) => path.endsWith(end)), end).toBe(true);
    }
  });

  it("hold no invisible character", () => {
    const found = Object.entries(sources).flatMap(([path, text]) =>
      text
        .split("\n")
        .flatMap((line, i) => ([...line].some((char) => invisible(char.codePointAt(0) ?? 0)) ? [`${path}:${i + 1}`] : [])),
    );
    expect(found).toEqual([]);
  });
});

describe("components", () => {
  it("never render raw HTML", () => {
    // Svelte escapes every {expression}; {@html} is the one way around it,
    // and would let a translation or a hostile host name inject markup.
    const raw = files.filter((file) => file.path.endsWith(".svelte") && file.text.includes("{@html"));
    expect(raw.map((file) => file.path)).toEqual([]);
  });
});
