import { describe, expect, it } from "vitest";
import { formatUuidV4, newId } from "./ids";

const V4 = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

describe("ids", () => {
  it("formats 16 bytes as a version-4 UUID", () => {
    expect(formatUuidV4(new Uint8Array(16))).toBe("00000000-0000-4000-8000-000000000000");
    expect(formatUuidV4(new Uint8Array(16).fill(0xff))).toBe("ffffffff-ffff-4fff-bfff-ffffffffffff");
    expect(() => formatUuidV4(new Uint8Array(3))).toThrow();
  });

  it("produces valid, distinct ids with or without crypto.randomUUID", () => {
    expect(newId()).toMatch(V4);
    const original = crypto.randomUUID;
    Object.defineProperty(crypto, "randomUUID", { value: undefined, configurable: true });
    try {
      const a = newId();
      const b = newId();
      expect(a).toMatch(V4);
      expect(a).not.toBe(b);
    } finally {
      Object.defineProperty(crypto, "randomUUID", { value: original, configurable: true });
    }
  });
});
