import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";

// Tauri expects a fixed port and fails the dev command rather than silently moving to
// another one — a moved port would leave the webview pointing at nothing.
export default defineConfig({
  plugins: [svelte(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // The Rust side has its own watcher; watching it here would restart Vite on
      // every cargo write.
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    // Matches `frontendDist` in tauri.conf.json.
    outDir: "dist",
    emptyOutDir: true,
    target: "safari15",
    sourcemap: process.env.TAURI_ENV_DEBUG === "true",
  },
});
