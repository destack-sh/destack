import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vite";
import tsconfigPaths from "vite-tsconfig-paths";

// https://vitejs.dev/config/
const defaultConfig = defineConfig(() => ({
  logLevel: "info",
  optimizeDeps: { exclude: ["destack"] },
  plugins: [
    tsconfigPaths({
      projects: ["./tsconfig.json", "../destack-ts/tsconfig.json"],
    }),
  ],
  resolve: {
    preserveSymlinks: true,
    alias: {
      "@destack": fileURLToPath(new URL("../destack-ts/src", import.meta.url)),
      "@destack-web": fileURLToPath(new URL("../destack-ts-web/src", import.meta.url)),
      "@desys": fileURLToPath(new URL("./src", import.meta.url)),
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
        assetFileNames: "assets/[name].[ext]",
        manualChunks: () => "everything.js",
      },
      onwarn(warning, warn) {
        warn(warning);
      },
    },
  },
}));
export default defaultConfig;
