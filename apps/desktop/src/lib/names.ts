/**
 * Split a browser name into what it is and how it starts.
 *
 * Detection names a private entry `Firefox (Private)`, which is too long
 * for a tile. Showing the part in brackets on its own line keeps both
 * readable, and a name without brackets is left alone.
 */
export function splitName(name: string): { base: string; mode: string | null } {
  const match = /^(.*\S)\s*\(([^()]+)\)\s*$/.exec(name.trim());
  if (!match) return { base: name.trim(), mode: null };
  return { base: match[1] ?? name, mode: match[2] ?? null };
}
