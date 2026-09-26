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

/** Tiles, a one-column list or a two-column list. */
export type PickerLayout = "tiles" | "list" | "two-columns";

export interface Settings {
  rules_enabled: boolean;
  open_under_cursor: boolean;
  close_on_focus_loss: boolean;
  offer_remember: boolean;
  theme: Theme;
  picker_layout: PickerLayout;
  /** Linux: stay running and start with the session, for a faster picker. */
  run_in_background: boolean;
  /** Ask once a day, at start-up, whether a newer version is published. */
  check_for_updates: boolean;
}

/** Where the running version stands; colours the picker's update button. */
export type UpdateStatus =
  | { state: "unknown" }
  | { state: "current"; version: string }
  | { state: "available"; version: string }
  | { state: "failed"; reason: string };

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
  update: UpdateStatus;
}

export interface TestResult {
  normalized: string | null;
  error: string | null;
  matched: { ruleset: string; pattern: string; browser: string; launch: string | null } | null;
}

export type RegisterOutcome = { kind: "done" } | { kind: "needs_user_action"; message: string };

/** What the settings window edits (`get_config`). */
export interface EditableConfig {
  config: Config;
  /** Why config.toml could not be read; `config` is then empty. */
  load_error: string | null;
}

export interface ImportReport {
  /** The draft with the import merged in, not saved yet. */
  config: Config;
  browsers_added: number;
  rulesets_added: number;
  notes: string[];
}

export interface LatestRelease {
  /** Published version, without any leading `v`. */
  version: string;
  /** Whether it is newer than the running one. */
  newer: boolean;
  /** Fixed download page; never a URL taken from a server answer. */
  page: string;
}

export interface AppInfo {
  version: string;
  config_path: string;
  platform: string;
}
