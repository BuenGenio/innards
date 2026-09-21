import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
// @ts-expect-error type error without @types/node package
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
// Design-mode helper: `/__snap-hold?ms=N` answers after N ms. app.html
// requests it as an <img> when `?snap=1` is set so a headless browser's
// load-event screenshot happens after the UI has rendered its data.
/** @type {import('vite').Plugin} */
const snapHold = {
  name: "innards-snap-hold",
  configureServer(server) {
    server.middlewares.use("/__snap-hold", (/** @type {any} */ req, /** @type {any} */ res) => {
      const ms = Number(new URL(req.url, "http://x").searchParams.get("ms") ?? 2000);
      setTimeout(() => {
        res.setHeader("Content-Type", "image/gif");
        res.end(Uint8Array.from(atob("R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7"), (c) => c.charCodeAt(0)));
      }, ms);
    });
  },
};

export default defineConfig(() => ({
  plugins: [sveltekit(), snapHold],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
