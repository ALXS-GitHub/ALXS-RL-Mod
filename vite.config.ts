import { fileURLToPath, URL } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// Tauri expects a fixed port and IPv4 loopback (the webview does not resolve
// `localhost` to ::1 reliably on Windows).
const DEV_HOST = "127.0.0.1";
const DEV_PORT = 1420;

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
  },
  clearScreen: false,
  server: {
    host: DEV_HOST,
    port: DEV_PORT,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**", "**/legacy/**", "**/docs_rl/**"] },
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    target: "es2022",
    sourcemap: false,
    chunkSizeWarningLimit: 1200,
  },
});
