import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

// Tauri expone esta variable cuando se desarrolla contra un dispositivo real.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],

  // Tauri usa una terminal propia; que Vite no la limpie evita perder sus logs.
  clearScreen: false,

  server: {
    // Puerto fijo: `devUrl` de tauri.conf.json apunta acá.
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: {
      // El watcher de Vite no tiene nada que hacer con el crate de Rust.
      ignored: ["**/src-tauri/**"],
    },
  },

  test: {
    include: ["src/**/*.test.ts"],
  },
});
