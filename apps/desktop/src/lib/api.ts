// Typed wrappers around the IPC commands. Keep the names in sync with
// `src-tauri/src/commands.rs` and `src-tauri/build.rs`.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { tRust } from "../i18n/i18n.svelte";
import type {
  AppInfo,
  Config,
  EditableConfig,
  ImportReport,
  LaunchContext,
  RegisterOutcome,
  LatestRelease,
  RustMessage,
  TestResult,
  UpdateStatus,
  UrlView,
  Uuid,
} from "./types";

export const api = {
  launchContext: () => invoke<LaunchContext>("launch_context"),
  /** `url` is the link on screen; Rust refuses the choice if another one
   * has arrived since. */
  pick: (url: string, browser: Uuid, launch: Uuid | null, remember: boolean) =>
    invoke<void>("pick", { url, browser, launch, remember }),
  /** Replace `url`, the link on screen, with `edited`, which Rust validates
   * as any incoming link; the link to show comes back. */
  editLink: (url: string, edited: string) => invoke<UrlView>("edit_link", { url, edited }),
  dismiss: () => invoke<void>("dismiss"),
  /** The picker's page has read the pending link; Rust shows the window. */
  showPicker: () => invoke<void>("show_picker"),
  openSettings: () => invoke<void>("open_settings"),

  getConfig: () => invoke<EditableConfig>("get_config"),
  saveConfig: (config: Config) => invoke<void>("save_config", { config }),
  /** Detection and import work on the unsaved draft and return it merged. */
  discoverBrowsers: (config: Config) => invoke<Config>("discover_browsers", { config }),
  testUrl: (url: string) => invoke<TestResult>("test_url", { url }),
  defaultBrowserStatus: () => invoke<boolean>("default_browser_status"),
  registerDefaultBrowser: () => invoke<RegisterOutcome>("register_default_browser"),
  importHurl: (config: Config, json: string) => invoke<ImportReport>("import_hurl", { config, json }),
  appInfo: () => invoke<AppInfo>("app_info"),

  /** Ask now which version is newest (an update button was pressed). */
  checkLatestRelease: () => invoke<LatestRelease>("check_latest_release"),
  /** Open the download page in a browser. */
  openReleasePage: () => invoke<void>("open_release_page"),

  /** Fired when a new link waits for the picker: a Reroute running in the
   * background, or a link macOS delivers to the open app. */
  onContextChanged: (handler: () => void): Promise<UnlistenFn> =>
    listen("context-changed", () => handler()),

  /** Fired when the automatic version check has an answer. */
  onUpdateStatus: (handler: (status: UpdateStatus) => void): Promise<UnlistenFn> =>
    listen<UpdateStatus>("update-status", (event) => handler(event.payload)),
};

function isRustMessage(value: unknown): value is RustMessage {
  if (typeof value !== "object" || value === null) return false;
  const { code, params, text } = value as Record<string, unknown>;
  return typeof code === "string" && typeof text === "string" && typeof params === "object" && params !== null;
}

/**
 * A thrown value as a message: a failed command throws the one Rust sent.
 * Anything else, such as the IPC bridge itself failing, keeps its own text,
 * untranslated, under a code no catalogue has.
 */
export function asMessage(error: unknown): RustMessage {
  if (isRustMessage(error)) return error;
  const text = typeof error === "string" ? error : error instanceof Error ? error.message : String(error);
  return { code: "", params: {}, text };
}

/** Turn any thrown value into a readable message, in the current language. */
export function describeError(error: unknown): string {
  return tRust(asMessage(error));
}
