import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Vite serves the UI during `tauri dev` and builds `dist/` for bundling.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: "127.0.0.1",
  },
  build: {
    target: ["es2022", "chrome110", "safari15"],
    sourcemap: false,
    minify: "esbuild",
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
