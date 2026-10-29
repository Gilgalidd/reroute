// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import Picker from "./Picker.svelte";
import { launchContext, mockIpc, rustMessage } from "../test/ipc";

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
    expect(pick?.args).toEqual({
      url: launchContext().url?.href,
      browser: "22222222-2222-2222-2222-222222222222",
      launch: null,
      remember: true,
    });
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

  it("offers to remember the domain only while rules are on", async () => {
    const base = launchContext();
    mockIpc({ launch_context: () => ({ ...base, settings: { ...base.settings, rules_enabled: false } }) });
    render(Picker);
    await screen.findByText("github.com", { selector: ".host" });
    expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
  });

  it("checks for a new version from the footer and offers the download page", async () => {
    const ipc = mockIpc({
      launch_context: () => launchContext(),
      check_latest_release: () => ({ version: "9.9.9", newer: true, page: "https://example.org/" }),
      open_release_page: () => null,
    });
    render(Picker);
    await screen.findByText("github.com", { selector: ".host" });
    expect(ipc.calls.some((c) => c.cmd === "check_latest_release")).toBe(false);
    await fireEvent.click(screen.getByRole("button", { name: "Check for a new version" }));
    await fireEvent.click(await screen.findByRole("button", { name: "Version 9.9.9 is available" }));
    expect(ipc.calls.some((c) => c.cmd === "open_release_page")).toBe(true);
  });

  it("says when the running version is the newest", async () => {
    mockIpc({
      launch_context: () => launchContext(),
      check_latest_release: () => ({ version: "0.1.11", newer: false, page: "https://example.org/" }),
    });
    render(Picker);
    await screen.findByText("github.com", { selector: ".host" });
    await fireEvent.click(screen.getByRole("button", { name: "Check for a new version" }));
    expect(await screen.findByText("Up to date (0.1.11)")).toBeInTheDocument();
  });

  it("colours the update button with what Reroute's own check found", async () => {
    const base = launchContext();
    mockIpc({ launch_context: () => ({ ...base, update: { state: "available", version: "9.9.9" } }) });
    render(Picker);
    await screen.findByText("github.com", { selector: ".host" });
    const button = screen.getByRole("button", { name: "Check for a new version" });
    expect(button).toHaveClass("available");
    expect(button).toHaveAttribute("title", "Version 9.9.9 is available");
  });

  it("asks to be shown once the link is on screen", async () => {
    const ipc = mockIpc({ launch_context: () => launchContext() });
    render(Picker);
    await screen.findByText("github.com", { selector: ".host" });
    const order = ipc.calls.map((c) => c.cmd).filter((cmd) => cmd === "launch_context" || cmd === "show_picker");
    expect(order).toEqual(["launch_context", "show_picker"]);
  });

  describe("quick actions", () => {
    const PRIVATE = "44444444-4444-4444-4444-444444444444";

    /** Firefox with a private entry, hidden from the picker. */
    function withPrivateFirefox() {
      const base = launchContext();
      return { ...base, browsers: base.browsers.map((b, i) => (i === 0 ? { ...b, private: PRIVATE } : b)) };
    }

    /** A Clipboard API that records what it is given. */
    function clipboard(writeText: (text: string) => Promise<void>) {
      Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    }

    afterEach(() => {
      Object.defineProperty(navigator, "clipboard", { value: undefined, configurable: true });
    });

    it("opens the private entry with Shift and a digit", async () => {
      const ipc = mockIpc({ launch_context: withPrivateFirefox, pick: () => null });
      render(Picker);
      await screen.findByText("github.com", { selector: ".host" });
      await fireEvent.keyDown(window, { key: "!", code: "Digit1", shiftKey: true });
      const picks = () => ipc.calls.filter((c) => c.cmd === "pick").map((c) => c.args.browser);
      expect(picks()).toEqual([PRIVATE]);
    });

    it("opens the private entry on a Shift-click, the browser itself without one", async () => {
      for (const [shiftKey, expected] of [
        [true, PRIVATE],
        [false, "11111111-1111-1111-1111-111111111111"],
      ] as const) {
        document.body.innerHTML = "";
        const ipc = mockIpc({ launch_context: withPrivateFirefox, pick: () => null });
        render(Picker);
        await fireEvent.click(await screen.findByRole("button", { name: /^Firefox/ }), { shiftKey });
        expect(ipc.calls.find((c) => c.cmd === "pick")?.args).toMatchObject({ browser: expected, launch: null });
      }
    });

    it("opens a browser that has no private entry as usual with Shift", async () => {
      const ipc = mockIpc({ launch_context: () => launchContext(), pick: () => null });
      render(Picker);
      await screen.findByText("github.com", { selector: ".host" });
      await fireEvent.keyDown(window, { key: "Enter", shiftKey: true });
      expect(ipc.calls.find((c) => c.cmd === "pick")?.args).toMatchObject({ browser: "11111111-1111-1111-1111-111111111111" });
    });

    it("copies the link and closes with Ctrl+C or the button", async () => {
      const copied: string[] = [];
      clipboard((text) => {
        copied.push(text);
        return Promise.resolve();
      });
      const ipc = mockIpc({ launch_context: () => launchContext(), dismiss: () => null });
      render(Picker);
      await screen.findByText("github.com", { selector: ".host" });
      await fireEvent.keyDown(window, { key: "c", ctrlKey: true });
      await waitFor(() => expect(ipc.calls.some((c) => c.cmd === "dismiss")).toBe(true));
      await fireEvent.click(screen.getByRole("button", { name: "Copy the link" }));
      await waitFor(() => expect(copied).toHaveLength(2));
      expect(copied).toEqual([launchContext().url?.href, launchContext().url?.href]);
      expect(ipc.calls.some((c) => c.cmd === "pick")).toBe(false);
    });

    it("stays open and says so when the link could not be copied", async () => {
      clipboard(() => Promise.reject(new Error("denied")));
      const ipc = mockIpc({ launch_context: () => launchContext(), dismiss: () => null });
      render(Picker);
      await screen.findByText("github.com", { selector: ".host" });
      await fireEvent.click(screen.getByRole("button", { name: "Copy the link" }));
      expect(await screen.findByRole("alert")).toHaveTextContent("The link could not be copied.");
      expect(ipc.calls.some((c) => c.cmd === "dismiss")).toBe(false);
    });

    it("edits the link with Ctrl+L, Rust checks it, and the browser gets the new one", async () => {
      const edited = { href: "https://example.org/other", host: "example.org" };
      const ipc = mockIpc({ launch_context: () => launchContext(), edit_link: () => edited, pick: () => null });
      render(Picker);
      await screen.findByText("github.com", { selector: ".host" });
      await fireEvent.keyDown(window, { key: "l", ctrlKey: true });
      const field = await screen.findByRole("textbox", { name: "Link to open" });
      expect(field).toHaveValue(launchContext().url?.href);

      // Keys typed in the field are the field's: no digit picks a browser.
      await fireEvent.keyDown(field, { key: "2" });
      await fireEvent.input(field, { target: { value: "https://example.org/other" } });
      await fireEvent.keyDown(field, { key: "Enter" });
      expect(await screen.findByText("example.org", { selector: ".host" })).toBeInTheDocument();
      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
      expect(ipc.calls.find((c) => c.cmd === "edit_link")?.args).toEqual({
        url: launchContext().url?.href,
        edited: "https://example.org/other",
      });
      expect(ipc.calls.some((c) => c.cmd === "pick")).toBe(false);

      await fireEvent.keyDown(window, { key: "1" });
      expect(ipc.calls.find((c) => c.cmd === "pick")?.args).toMatchObject({ url: edited.href });
    });

    it("keeps the field open on a refused link, and Escape gives the old one back", async () => {
      const ipc = mockIpc({
        launch_context: () => launchContext(),
        edit_link: () => {
          throw rustMessage("url.scheme_not_allowed", { scheme: "javascript" });
        },
        dismiss: () => null,
      });
      render(Picker);
      await screen.findByText("github.com", { selector: ".host" });
      await fireEvent.click(screen.getByRole("button", { name: "Edit the link" }));
      const field = await screen.findByRole("textbox", { name: "Link to open" });
      await fireEvent.input(field, { target: { value: "javascript:alert(1)" } });
      await fireEvent.keyDown(field, { key: "Enter" });
      expect(await screen.findByRole("alert")).toHaveTextContent("“javascript” links are not allowed");
      expect(field).toBeInTheDocument();

      await fireEvent.keyDown(field, { key: "Escape" });
      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
      expect(screen.queryByRole("alert")).not.toBeInTheDocument();
      expect(screen.getByText(launchContext().url?.href ?? "", { selector: ".href" })).toBeInTheDocument();
      expect(ipc.calls.some((c) => c.cmd === "dismiss")).toBe(false);
    });

    it("lists the new keys in its hint", async () => {
      mockIpc({ launch_context: () => launchContext() });
      render(Picker);
      await screen.findByText("github.com", { selector: ".host" });
      expect(screen.getByRole("status")).toHaveTextContent("Shift private · Ctrl+C copy · Ctrl+L edit");
    });
  });

  it("dismisses on Escape and shows a refused URL", async () => {
    const ipc = mockIpc({
      launch_context: () => ({ ...launchContext(), url: null, url_error: rustMessage("url.scheme_not_allowed", { scheme: "javascript" }) }),
      dismiss: () => null,
    });
    render(Picker);
    expect(await screen.findByRole("alert")).toHaveTextContent("This link was refused: “javascript” links are not allowed");
    await fireEvent.keyDown(window, { key: "Escape" });
    expect(ipc.calls.some((c) => c.cmd === "dismiss")).toBe(true);
  });
});
