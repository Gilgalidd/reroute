// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { copyText } from "./clipboard";

/** Give the page a Clipboard API, or take it away with `undefined`. */
function clipboard(value: { writeText: (text: string) => Promise<void> } | undefined) {
  Object.defineProperty(navigator, "clipboard", { value, configurable: true });
}

/** Give the document the older `copy` command, answering `result`. */
function execCommand(result: boolean) {
  const command = vi.fn(() => result);
  Object.defineProperty(document, "execCommand", { value: command, configurable: true });
  return command;
}

describe("copyText", () => {
  afterEach(() => {
    clipboard(undefined);
    Object.defineProperty(document, "execCommand", { value: undefined, configurable: true });
  });

  it("writes through the Clipboard API", async () => {
    const writeText = vi.fn(() => Promise.resolve());
    clipboard({ writeText });
    const command = execCommand(true);
    await copyText("https://a.org/");
    expect(writeText).toHaveBeenCalledWith("https://a.org/");
    expect(command).not.toHaveBeenCalled();
  });

  it("falls back to the copy command, leaving no field behind", async () => {
    clipboard({ writeText: () => Promise.reject(new Error("not allowed")) });
    const command = execCommand(true);
    await copyText("https://a.org/");
    expect(command).toHaveBeenCalledWith("copy");
    expect(document.querySelector("textarea")).toBeNull();
  });

  it("fails when nothing could copy", async () => {
    execCommand(false);
    await expect(copyText("https://a.org/")).rejects.toThrow();
  });
});
