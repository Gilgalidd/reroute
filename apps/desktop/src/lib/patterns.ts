// Client-side helpers for editing `kind:pattern` strings. Validation proper
// happens in Rust when the configuration is saved; this only drives the UI.

export type PatternKind = "exact" | "domain" | "regex";

export const PATTERN_KINDS: PatternKind[] = ["exact", "domain", "regex"];

export interface ParsedPattern {
  kind: PatternKind;
  body: string;
}

/** Split `kind:body`; a missing or unknown prefix means `exact`, like Rust. */
export function parsePattern(text: string): ParsedPattern {
  const trimmed = text.trim();
  const colon = trimmed.indexOf(":");
  if (colon > 0) {
    const kind = trimmed.slice(0, colon);
    const body = trimmed.slice(colon + 1).trim();
    if ((PATTERN_KINDS as string[]).includes(kind) && !body.startsWith("//")) {
      return { kind: kind as PatternKind, body };
    }
  }
  return { kind: "exact", body: trimmed };
}

export function formatPattern(p: ParsedPattern): string {
  return `${p.kind}:${p.body}`;
}

/** One pattern per line, blanks ignored. */
export function splitLines(text: string): string[] {
  return text
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter((l) => l.length > 0);
}

/** Cheap client-side sanity check, mirroring the Rust rules. */
export function patternProblem(text: string): string | null {
  const { kind, body } = parsePattern(text);
  if (!body) return "empty pattern";
  if (kind === "domain") {
    const host = body.startsWith("*.") ? body.slice(2) : body;
    if (!/^[a-z0-9-]+(\.[a-z0-9-]+)*$/i.test(host)) return "not a valid host name";
  }
  if (kind === "regex") {
    try {
      new RegExp(body);
    } catch {
      return "not a valid regular expression";
    }
  }
  return null;
}
