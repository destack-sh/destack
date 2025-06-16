import { fileURLToPath, URL } from "node:url";

import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://vitejs.dev/config/
const defaultConfig = defineConfig(() => ({
  logLevel: "info",
  plugins: [react()],
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
    sourcemap: true,
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
