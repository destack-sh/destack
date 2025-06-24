import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

// https://vitejs.dev/config/
const defaultConfig = defineConfig({
  logLevel: "info",
  plugins: [],
  resolve: {
    preserveSymlinks: true,
    alias: {
      "@destack": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    sourcemap: true,
  },
});

export default defaultConfig;
