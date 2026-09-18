// Shared Vitest setup: jest-dom matchers such as toBeInTheDocument, and a
// ResizeObserver stub because jsdom has none (Svelte's bind:clientWidth
// needs it). Component tests install a fake Tauri IPC bridge with `mockIpc`
// from ./ipc.
import "@testing-library/jest-dom/vitest";

if (typeof globalThis.ResizeObserver === "undefined") {
  class ResizeObserverStub {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  }
  globalThis.ResizeObserver = ResizeObserverStub as unknown as typeof ResizeObserver;
}
