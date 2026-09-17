// Mirrors the Rust types serialised over IPC (see crates/core and commands.rs).

export type Uuid = string;

export interface Launch {
  id: Uuid;
  name: string;
  args: string[];
}

export interface Browser {
  id: Uuid;
  name: string;
  path: string;
  args: string[];
  hidden: boolean;
  icon?: string | null;
  launches: Launch[];
}

export interface Ruleset {
  name: string;
  browser: Uuid;
  launch?: Uuid | null;
  /** `kind:pattern` strings, see docs/configuration.md. */
  patterns: string[];
}

export type Theme = "auto" | "light" | "dark";

export interface Settings {
  rules_enabled: boolean;
  open_under_cursor: boolean;
  close_on_focus_loss: boolean;
  offer_remember: boolean;
  theme: Theme;
}

export interface Config {
  version: number;
  settings: Settings;
  browsers: Browser[];
  rulesets: Ruleset[];
}

export interface Brand {
  key: string;
  color: string;
}

export interface BrowserView {
  id: Uuid;
  name: string;
  launches: { id: Uuid; name: string }[];
  icon: string | null;
  brand: Brand | null;
}

export interface LaunchContext {
  url: { href: string; host: string } | null;
  url_error: string | null;
  config_error: string | null;
  settings: Settings;
  browsers: BrowserView[];
}

export interface TestResult {
  normalized: string | null;
  error: string | null;
  matched: { ruleset: string; pattern: string; browser: string; launch: string | null } | null;
}

export type RegisterOutcome = { kind: "done" } | { kind: "needs_user_action"; message: string };

export interface ImportReport {
  browsers_added: number;
  rulesets_added: number;
  notes: string[];
}

export interface AppInfo {
  version: string;
  config_path: string;
  platform: string;
}
