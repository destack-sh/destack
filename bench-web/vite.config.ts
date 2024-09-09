import { fileURLToPath, URL } from "node:url";

import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import vueJsx from "@vitejs/plugin-vue-jsx";

// https://vitejs.dev/config/
const defaultConfig = defineConfig(({ mode }) => ({
  logLevel: "info",
  plugins: [vue(), vueJsx()],
  resolve: {
    preserveSymlinks: true,
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    chunkSizeWarningLimit: 8 * 1024,
    minify: mode !== "unminified",
    rollupOptions: {
      output: {
        // bundle everything into one file
        entryFileNames: "assets/[name].js",
        chunkFileNames: "assets/[name].js",
        assetFileNames: "assets/[name].[ext]",
        manualChunks: () => "everything.js",
      },
      onwarn(warning, warn) {
        if (warning.message.includes("but also statically imported by")) {
          return;
        }
        warn(warning);
      },
    },
  },
}));
export default defaultConfig;
