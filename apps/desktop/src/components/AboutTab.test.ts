// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it } from "vitest";
import AboutTab from "./AboutTab.svelte";
import { mockIpc } from "../test/ipc";

const PAGE = "https://github.com/Gilgalidd/reroute/releases/latest";

function show(handlers: Record<string, () => unknown> = {}) {
  const ipc = mockIpc({
    app_info: () => ({ version: "0.1.2", config_path: "/tmp/config.toml", platform: "linux" }),
    ...handlers,
  });
  render(AboutTab);
  return ipc;
}

describe("About tab, version check", () => {
  beforeEach(() => {
    document.body.innerHTML = "";
  });

  it("says nothing until the button is pressed", async () => {
    const ipc = show();
    await screen.findByRole("button", { name: "Check for a new version" });
    expect(ipc.calls.some((c) => c.cmd === "check_latest_release")).toBe(false);
  });

  it("reports that the running version is the newest", async () => {
    show({ check_latest_release: () => ({ version: "0.1.2", newer: false, page: PAGE }) });
    await fireEvent.click(screen.getByRole("button", { name: "Check for a new version" }));
    expect(await screen.findByText(/You have the latest version \(0\.1\.2\)/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Open the download page" })).toBeNull();
  });

  it("offers the download page when a newer version exists", async () => {
    const ipc = show({
      check_latest_release: () => ({ version: "0.2.0", newer: true, page: PAGE }),
      open_release_page: () => null,
    });
    await fireEvent.click(screen.getByRole("button", { name: "Check for a new version" }));
    expect(await screen.findByText(/is available/)).toBeInTheDocument();

    await fireEvent.click(screen.getByRole("button", { name: "Open the download page" }));
    await waitFor(() => expect(ipc.calls.some((c) => c.cmd === "open_release_page")).toBe(true));
  });

  it("shows why a check failed", async () => {
    show({
      check_latest_release: () => {
        throw "no published release was found (are the releases private?)";
      },
    });
    await fireEvent.click(screen.getByRole("button", { name: "Check for a new version" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("no published release was found");
  });
});
