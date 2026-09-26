// `vitest/config` is Vite's defineConfig with the `test` section typed.
import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { svelteTesting } from "@testing-library/svelte/vite";

// Vite serves the UI during `tauri dev` and builds `dist/` for bundling.
export default defineConfig({
  plugins: [svelte(), svelteTesting()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: "127.0.0.1",
  },
  build: {
    target: ["es2022", "chrome110", "safari15"],
    // Minified by Vite's default (Oxc since Vite 8, which ships no esbuild).
    sourcemap: false,
  },
  test: {
    // Pure helpers run in node; component tests opt into jsdom with a
    // `// @vitest-environment jsdom` header.
    environment: "node",
    include: ["src/**/*.test.ts"],
    setupFiles: ["src/test/setup.ts"],
  },
});
