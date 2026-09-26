import type { Config, LaunchContext } from "../lib/types";

type Handler = (args: Record<string, unknown>) => unknown;

/**
 * Install a fake `window.__TAURI_INTERNALS__` so that `@tauri-apps/api`
 * calls reach the given handlers instead of a Tauri process. Returns the
 * list of `save_config` payloads and a spy of every call.
 */
export function mockIpc(handlers: Record<string, Handler>) {
  const calls: { cmd: string; args: Record<string, unknown> }[] = [];
  const saved: Config[] = [];
  const bridge = {
    transformCallback: (cb: (payload: unknown) => void) => {
      const id = Math.floor(Math.random() * 1e9);
      (window as unknown as Record<string, unknown>)[`_${id}`] = cb;
      return id;
    },
    invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
      calls.push({ cmd, args });
      if (cmd === "save_config") {
        saved.push(JSON.parse(JSON.stringify(args.config)) as Config);
        return null;
      }
      if (cmd === "plugin:event|listen") return 1;
      if (cmd === "plugin:event|unlisten") return null;
      const handler = handlers[cmd];
      if (!handler) throw new Error(`unmocked command ${cmd}`);
      return handler(args);
    },
  };
  const w = window as unknown as Record<string, unknown>;
  w.__TAURI_INTERNALS__ = bridge;
  // `listen()` from @tauri-apps/api/event unregisters through this object.
  w.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => undefined };
  return { calls, saved };
}

/** A configuration as Rust serialises it: empty collections are omitted. */
export function sparseConfig(): Config {
  return {
    version: 1,
    settings: { rules_enabled: true, open_under_cursor: false, close_on_focus_loss: true, offer_remember: true, theme: "auto", picker_layout: "tiles" },
    browsers: [
      { id: "11111111-1111-1111-1111-111111111111", name: "Firefox", path: "/snap/bin/firefox", args: ["%URL%"], hidden: false } as Config["browsers"][number],
      {
        id: "22222222-2222-2222-2222-222222222222",
        name: "Chromium",
        path: "/snap/bin/chromium",
        args: ["%URL%"],
        hidden: false,
        launches: [{ id: "33333333-3333-3333-3333-333333333333", name: "Incognito", args: ["--incognito"] }],
      },
    ],
    rulesets: [{ name: "Work", browser: "22222222-2222-2222-2222-222222222222", patterns: ["domain:*.github.com"] } as Config["rulesets"][number]],
  };
}

export function launchContext(): LaunchContext {
  const config = sparseConfig();
  return {
    url: { href: "https://github.com/U-C-S/Hurl", host: "github.com" },
    url_error: null,
    config_error: null,
    settings: config.settings,
    browsers: config.browsers.map((b) => ({
      id: b.id,
      name: b.name,
      launches: (b.launches ?? []).map((l) => ({ id: l.id, name: l.name })),
      icon: null,
      brand: null,
    })),
  };
}
