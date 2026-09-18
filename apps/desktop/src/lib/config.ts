import type { Config } from "./types";

/**
 * Fill in the collections Rust omits when they are empty (`launches`,
 * `patterns`, …) so that the editors can push into them without checks.
 * Returns a deep copy; the input is left untouched.
 */
export function normalizeConfig(input: Config): Config {
  const config = JSON.parse(JSON.stringify(input)) as Config;
  config.browsers = (config.browsers ?? []).map((b) => ({
    ...b,
    args: b.args ?? [],
    hidden: b.hidden ?? false,
    icon: b.icon ?? null,
    launches: (b.launches ?? []).map((l) => ({ ...l, args: l.args ?? [] })),
  }));
  config.rulesets = (config.rulesets ?? []).map((r) => ({
    ...r,
    launch: r.launch ?? null,
    patterns: r.patterns ?? [],
  }));
  return config;
}
