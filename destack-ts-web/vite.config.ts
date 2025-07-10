import { fileURLToPath, URL } from "node:url";

import react from "@vitejs/plugin-react";
import { dirname, join } from "node:path";
import { defineConfig } from "vite";
import { viteStaticCopy } from "vite-plugin-static-copy";
import tsconfigPaths from "vite-tsconfig-paths";

const PYODIDE_EXCLUDE = ["!**/*.{md,html}", "!**/*.d.ts", "!**/*.whl", "!**/node_modules"];

export function viteStaticCopyPyodide() {
  const pyodideDir = dirname(fileURLToPath(import.meta.resolve("pyodide")));
  return viteStaticCopy({
    targets: [
      {
        src: [join(pyodideDir, "*")].concat(PYODIDE_EXCLUDE),
        dest: "assets",
      },
    ],
  });
}

const ReactCompilerConfig = {
  /* ... */
};

const PreactSignalsReactTransformConfig = {
  mode: "all",
};

// https://vitejs.dev/config/
const defaultConfig = defineConfig(() => ({
  logLevel: "info",
  optimizeDeps: { exclude: ["pyodide", "destack"] },
  plugins: [
    react({
      babel: {
        presets: [
          ["@babel/preset-typescript", { allowDeclareFields: true }],
          "@babel/preset-react",
        ],
        compact: true,
        plugins: [
          ["module:@preact/signals-react-transform", PreactSignalsReactTransformConfig],
          ["@babel/plugin-proposal-decorators", { version: "2023-11" }],
          // TODO :Performance: enable react compiler when it plays nicely with signals
          // ["babel-plugin-react-compiler", ReactCompilerConfig],
        ],
      },
    }),
    viteStaticCopyPyodide(),
    tsconfigPaths({
      projects: ["./tsconfig.json", "../destack-ts/tsconfig.json"],
    }),
  ],
  resolve: {
    preserveSymlinks: true,
    alias: {
      "@destack": fileURLToPath(new URL("../destack-ts/src", import.meta.url)),
      "@destack-web": fileURLToPath(new URL("./src", import.meta.url)),
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
