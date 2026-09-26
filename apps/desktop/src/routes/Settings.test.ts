// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it } from "vitest";
import Settings from "./Settings.svelte";
import { mockIpc, sparseConfig } from "../test/ipc";
import type { Config } from "../lib/types";

/** `config` plus one browser, as detection or an import would return it. */
function withBrowser(config: Config, name: string): Config {
  const id = `99999999-9999-9999-9999-${String(config.browsers.length).padStart(12, "0")}`;
  return { ...config, browsers: [...config.browsers, { id, name, path: "/usr/bin/x", args: [], hidden: false, launches: [] }] };
}

function setup() {
  const ipc = mockIpc({
    get_config: () => sparseConfig(),
    app_info: () => ({ version: "0.1.0", config_path: "/tmp/config.toml", platform: "linux" }),
    default_browser_status: () => true,
  });
  render(Settings);
  return ipc;
}

async function save() {
  const button = screen.getByRole("button", { name: "Save" });
  await waitFor(() => expect(button).not.toBeDisabled());
  await fireEvent.click(button);
}

describe("Settings window", () => {
  beforeEach(() => {
    document.body.innerHTML = "";
  });

  it("adds a launch option to a browser that had none", async () => {
    const ipc = setup();
    await fireEvent.click(await screen.findByRole("option", { name: "Firefox" }));
    await fireEvent.click(screen.getByRole("button", { name: "Add launch option" }));
    const [name, args] = screen.getAllByPlaceholderText(/Private window|--private-window/);
    await fireEvent.input(name!, { target: { value: "Kiosk" } });
    await fireEvent.input(args!, { target: { value: "--kiosk %URL%" } });
    await save();
    await waitFor(() => expect(ipc.saved).toHaveLength(1));
    expect(ipc.saved[0]?.browsers[0]?.launches).toEqual([
      { id: expect.stringMatching(/^[0-9a-f-]{36}$/), name: "Kiosk", args: ["--kiosk", "%URL%"] },
    ]);
  });

  it("saves the ruleset name, launch option and patterns", async () => {
    const ipc = setup();
    await fireEvent.click(await screen.findByRole("button", { name: "Rules" }));
    await fireEvent.input(screen.getByPlaceholderText("Ruleset name"), { target: { value: "Perso" } });
    await fireEvent.change(screen.getByLabelText("Launch option"), { target: { value: "33333333-3333-3333-3333-333333333333" } });
    await fireEvent.input(screen.getByLabelText("Patterns, one per line"), { target: { value: "domain:*.example.org\nregex:^https://x" } });
    await save();
    await waitFor(() => expect(ipc.saved).toHaveLength(1));
    expect(ipc.saved[0]?.rulesets[0]).toEqual({
      name: "Perso",
      browser: "22222222-2222-2222-2222-222222222222",
      launch: "33333333-3333-3333-3333-333333333333",
      patterns: ["domain:*.example.org", "regex:^https://x"],
    });
  });

  it("saves the picker layout chosen in the General tab", async () => {
    const ipc = setup();
    await fireEvent.click(await screen.findByRole("button", { name: "General" }));
    await fireEvent.change(screen.getByLabelText("Show browsers as"), { target: { value: "list" } });
    await save();
    await waitFor(() => expect(ipc.saved).toHaveLength(1));
    expect(ipc.saved[0]?.settings.picker_layout).toBe("list");
  });

  it("detects browsers into the draft, keeping unsaved edits, and saves only on Save", async () => {
    const ipc = setup();
    ipc.handlers.discover_browsers = (args) => withBrowser(args.config as Config, "Brave");
    await fireEvent.click(await screen.findByRole("option", { name: "Firefox" }));
    await fireEvent.input(screen.getByLabelText("Name"), { target: { value: "Firefox, edited" } });
    await fireEvent.click(screen.getByRole("button", { name: "Detect installed" }));
    expect(await screen.findByText(/Added 1 browser/)).toBeInTheDocument();
    expect(ipc.saved).toHaveLength(0);
    await save();
    await waitFor(() => expect(ipc.saved).toHaveLength(1));
    expect(ipc.saved[0]?.browsers.map((b) => b.name)).toEqual(["Firefox, edited", "Chromium", "Brave"]);
  });

  it("keeps a Hurl import through the next Save", async () => {
    const ipc = setup();
    ipc.handlers.import_hurl = (args) => ({
      config: withBrowser(args.config as Config, "Edge"),
      browsers_added: 1,
      rulesets_added: 0,
      notes: [],
    });
    await fireEvent.click(await screen.findByRole("button", { name: "General" }));
    await fireEvent.input(screen.getByPlaceholderText(/Browsers/), { target: { value: "{}" } });
    await fireEvent.click(screen.getByRole("button", { name: "Import" }));
    expect(await screen.findByText(/Imported 1 browser/)).toBeInTheDocument();
    await fireEvent.change(screen.getByLabelText("Show browsers as"), { target: { value: "list" } });
    await save();
    await waitFor(() => expect(ipc.saved).toHaveLength(1));
    expect(ipc.saved[0]?.browsers.map((b) => b.name)).toContain("Edge");
    expect(ipc.saved[0]?.settings.picker_layout).toBe("list");
  });

  it("shows unexpected errors instead of swallowing them", async () => {
    setup();
    await screen.findByRole("option", { name: "Firefox" });
    window.dispatchEvent(new ErrorEvent("error", { message: "boom" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Unexpected error: boom");
  });
});
