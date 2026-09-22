// Typed wrappers around the IPC commands. Keep the names in sync with
// `src-tauri/src/commands.rs` and `src-tauri/build.rs`.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppInfo,
  Config,
  ImportReport,
  LaunchContext,
  RegisterOutcome,
  LatestRelease,
  TestResult,
  Uuid,
} from "./types";

export const api = {
  launchContext: () => invoke<LaunchContext>("launch_context"),
  pick: (browser: Uuid, launch: Uuid | null, remember: boolean) =>
    invoke<void>("pick", { browser, launch, remember }),
  dismiss: () => invoke<void>("dismiss"),
  openSettings: () => invoke<void>("open_settings"),

  getConfig: () => invoke<Config>("get_config"),
  saveConfig: (config: Config) => invoke<void>("save_config", { config }),
  discoverBrowsers: () => invoke<Config>("discover_browsers"),
  testUrl: (url: string) => invoke<TestResult>("test_url", { url }),
  defaultBrowserStatus: () => invoke<boolean>("default_browser_status"),
  registerDefaultBrowser: () => invoke<RegisterOutcome>("register_default_browser"),
  importHurl: (json: string) => invoke<ImportReport>("import_hurl", { json }),
  appInfo: () => invoke<AppInfo>("app_info"),

  /** The only call that reaches the network, and only on demand. */
  checkLatestRelease: () => invoke<LatestRelease>("check_latest_release"),
  /** Open the download page in a browser. */
  openReleasePage: () => invoke<void>("open_release_page"),

  /** Fired by the Rust side when the pending URL changes (macOS). */
  onContextChanged: (handler: () => void): Promise<UnlistenFn> =>
    listen("context-changed", () => handler()),
};

/** Turn any thrown value into a readable message. */
export function describeError(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}
