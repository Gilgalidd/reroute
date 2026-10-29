// How complete each translation is: `npm run i18n:status`. On GitHub
// Actions the report also goes to the run's summary page. A missing
// translation is not an error (the text shows in English), so nothing here
// fails because of one; see docs/translations.md.
import { appendFileSync, readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const SOURCE = "en";

/** `key` without its plural `#category`. */
function base(key) {
  return key.split("#")[0];
}

/**
 * For each language but the source, sorted by tag: how many of the source's
 * texts it has, a plural counting once through its `#other` form, and which
 * ones it lacks, in the source's order.
 *
 * @param {Record<string, string>} source
 * @param {Record<string, Record<string, string>>} catalogs by language tag
 */
export function status(source, catalogs) {
  const keys = Object.keys(source);
  const texts = [...new Set(keys.map(base))];
  const plurals = new Set(keys.filter((key) => key.includes("#")).map(base));
  return Object.keys(catalogs)
    .filter((tag) => tag !== SOURCE)
    .sort()
    .map((tag) => {
      const catalog = catalogs[tag];
      const missing = texts.filter((text) => catalog[plurals.has(text) ? `${text}#other` : text] === undefined);
      return {
        tag,
        name: catalog["language.name"] ?? tag,
        total: texts.length,
        translated: texts.length - missing.length,
        missing,
      };
    });
}

/** Text from a translation file, as plain text in Markdown. */
function plain(text) {
  return text.replace(/[\\`*_[\]<>|]/g, (mark) => `\\${mark}`);
}

/**
 * The report, in Markdown, which reads as well in a terminal as on GitHub.
 *
 * @param {ReturnType<typeof status>} languages
 */
export function report(languages) {
  const lines = ["## Translations", ""];
  if (languages.length === 0) return [...lines, "English only.", ""].join("\n");
  lines.push("| Language | Translated | Missing |", "|---|---:|---:|");
  for (const language of languages) {
    // Rounded down: 99.6 % is not complete.
    const share = Math.floor((language.translated * 100) / language.total);
    lines.push(
      `| ${plain(language.name)} (\`${language.tag}\`) | ${share}% (${language.translated} of ${language.total}) | ${language.missing.length} |`,
    );
  }
  lines.push("");
  const incomplete = languages.filter((language) => language.missing.length > 0);
  if (incomplete.length === 0) {
    lines.push("Every language is complete.");
  } else {
    lines.push("### Missing translations", "", "These show in English until someone translates them.");
    for (const language of incomplete) {
      const keys = language.missing.map((key) => `\`${key}\``).join(", ");
      lines.push("", `${plain(language.name)} (\`${language.tag}\`): ${keys}`);
    }
  }
  lines.push("");
  return lines.join("\n");
}

function main() {
  const dir = new URL("../src/i18n/locales/", import.meta.url);
  const catalogs = {};
  for (const file of readdirSync(dir).sort()) {
    // Only names a language tag can have: the tag goes into the report.
    const tag = /^([A-Za-z0-9-]+)\.json$/.exec(file)?.[1];
    if (tag) catalogs[tag] = JSON.parse(readFileSync(new URL(file, dir), "utf8"));
  }
  const text = report(status(catalogs[SOURCE] ?? {}, catalogs));
  process.stdout.write(text);
  const summary = process.env.GITHUB_STEP_SUMMARY;
  if (summary) appendFileSync(summary, text);
}

// Run as a script, not when a test imports the functions above.
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
