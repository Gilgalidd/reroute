// @vitest-environment jsdom
import { fireEvent, render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it } from "vitest";
import Picker from "./Picker.svelte";
import { launchContext, mockIpc } from "../test/ipc";

describe("Picker window", () => {
  beforeEach(() => {
    document.body.innerHTML = "";
  });

  it("shows the host and one tile per browser", async () => {
    mockIpc({ launch_context: () => launchContext() });
    render(Picker);
    expect(await screen.findByText("github.com", { selector: ".host" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /^Firefox/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /^Chromium, has launch options/ })).toBeInTheDocument();
  });

  it("picks a browser with a digit key and remembers the domain when asked", async () => {
    const ipc = mockIpc({ launch_context: () => launchContext(), pick: () => null });
    render(Picker);
    await screen.findByText("github.com", { selector: ".host" });
    await fireEvent.click(screen.getByRole("checkbox"));
    await fireEvent.keyDown(window, { key: "2" });
    const pick = ipc.calls.find((c) => c.cmd === "pick");
    expect(pick?.args).toEqual({ browser: "22222222-2222-2222-2222-222222222222", launch: null, remember: true });
  });

  it("numbers only the tiles a digit key can pick", async () => {
    const base = launchContext();
    const browsers = Array.from({ length: 11 }, (_, i) => ({
      ...base.browsers[0]!,
      id: `00000000-0000-0000-0000-${String(i).padStart(12, "0")}`,
      name: `Browser ${i + 1}`,
    }));
    mockIpc({ launch_context: () => ({ ...base, browsers }) });
    const { container } = render(Picker);
    await screen.findByRole("button", { name: "Browser 11" });
    const badges = Array.from(container.querySelectorAll(".badge"), (b) => b.textContent);
    expect(badges).toEqual(["1", "2", "3", "4", "5", "6", "7", "8", "9", "", ""]);
  });

  it("lists browsers in two columns, the arrows moving by row", async () => {
    const base = launchContext();
    const browsers = ["A", "B", "C", "D"].map((name, i) => ({
      ...base.browsers[0]!,
      id: `00000000-0000-0000-0000-00000000000${i}`,
      name,
    }));
    const ipc = mockIpc({
      launch_context: () => ({ ...base, browsers, settings: { ...base.settings, picker_layout: "two-columns" } }),
      pick: () => null,
    });
    const { container } = render(Picker);
    await screen.findByRole("button", { name: "D" });
    expect(container.querySelector(".grid")).toHaveClass("two-columns");
    expect(container.querySelectorAll(".cell.row")).toHaveLength(4);
    await fireEvent.keyDown(window, { key: "ArrowDown" });
    await fireEvent.keyDown(window, { key: "Enter" });
    const pick = ipc.calls.find((c) => c.cmd === "pick");
    expect(pick?.args).toMatchObject({ browser: browsers[2]!.id });
  });

  it("opens the launch menu from the chevron without opening the browser", async () => {
    const ipc = mockIpc({ launch_context: () => launchContext(), pick: () => null });
    render(Picker);
    await fireEvent.click(await screen.findByRole("button", { name: "Launch options for Chromium" }));
    expect(screen.getByRole("menu")).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "Incognito" })).toBeInTheDocument();
    expect(ipc.calls.some((c) => c.cmd === "pick")).toBe(false);
  });

  it("dismisses on Escape and shows a refused URL", async () => {
    const ipc = mockIpc({
      launch_context: () => ({ ...launchContext(), url: null, url_error: "scheme `javascript` is not allowed" }),
      dismiss: () => null,
    });
    render(Picker);
    expect(await screen.findByRole("alert")).toHaveTextContent("scheme `javascript` is not allowed");
    await fireEvent.keyDown(window, { key: "Escape" });
    expect(ipc.calls.some((c) => c.cmd === "dismiss")).toBe(true);
  });
});
