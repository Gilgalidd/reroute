import { describe, expect, it } from "vitest";
import { normalizeConfig } from "./config";
import type { Config } from "./types";

describe("normalizeConfig", () => {
  it("fills in collections that Rust omits when empty", () => {
    const sparse = {
      version: 1,
      settings: { rules_enabled: true, open_under_cursor: false, close_on_focus_loss: true, offer_remember: true, theme: "auto" },
      browsers: [{ id: "a", name: "Firefox", path: "/usr/bin/firefox" }],
      rulesets: [{ name: "Work", browser: "a" }],
    } as unknown as Config;
    const full = normalizeConfig(sparse);
    expect(full.browsers[0]?.launches).toEqual([]);
    expect(full.browsers[0]?.args).toEqual([]);
    expect(full.browsers[0]?.hidden).toBe(false);
    expect(full.browsers[0]?.icon).toBeNull();
    expect(full.rulesets[0]?.patterns).toEqual([]);
    expect(full.rulesets[0]?.launch).toBeNull();
    expect((sparse.browsers[0] as { launches?: unknown }).launches).toBeUndefined();
  });

  it("keeps existing values", () => {
    const config = {
      version: 1,
      settings: { rules_enabled: false, open_under_cursor: true, close_on_focus_loss: false, offer_remember: false, theme: "dark" },
      browsers: [{ id: "a", name: "F", path: "/f", args: ["-x"], hidden: true, icon: "/i.png", launches: [{ id: "l", name: "P", args: ["-p"] }] }],
      rulesets: [{ name: "W", browser: "a", launch: "l", patterns: ["domain:x.org"] }],
    } as Config;
    expect(normalizeConfig(config)).toEqual(config);
  });
});
