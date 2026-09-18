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
