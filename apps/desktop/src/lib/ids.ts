/**
 * Fresh UUID v4 for browsers and launches created in the settings UI.
 *
 * `crypto.randomUUID` only exists in secure contexts, and WebKitGTK does not
 * treat the dev server's `http://127.0.0.1` origin as one, so fall back to
 * building the UUID from `getRandomValues`, which is always available.
 */
export function newId(): string {
  if (typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  return formatUuidV4(bytes);
}

/** RFC 4122 version-4 layout over 16 random bytes. */
export function formatUuidV4(bytes: Uint8Array): string {
  if (bytes.length !== 16) throw new Error("a UUID needs 16 bytes");
  const b = Uint8Array.from(bytes);
  b[6] = ((b[6] ?? 0) & 0x0f) | 0x40;
  b[8] = ((b[8] ?? 0) & 0x3f) | 0x80;
  const hex = Array.from(b, (x) => x.toString(16).padStart(2, "0")).join("");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}
