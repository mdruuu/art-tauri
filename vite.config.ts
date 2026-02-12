import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { resolve } from "path";

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    rollupOptions: {
      input: {
        overlay: resolve(__dirname, "src/overlay.html"),
        settings: resolve(__dirname, "src/settings.html"),
      },
    },
    outDir: "dist",
  },
});
