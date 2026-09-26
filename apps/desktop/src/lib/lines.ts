/**
 * One item per line, trimmed, blank lines dropped. The settings use it for
 * every list typed as text: rule patterns and command-line arguments. One
 * argument per line is what lets an argument contain a space, as in
 * `--profile-directory=Profile 1`.
 */
export function splitLines(text: string): string[] {
  return text
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter((l) => l.length > 0);
}
