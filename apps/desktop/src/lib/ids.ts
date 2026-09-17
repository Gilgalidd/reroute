/** Fresh UUID v4 for browsers and launches created in the settings UI. */
export function newId(): string {
  return crypto.randomUUID();
}
