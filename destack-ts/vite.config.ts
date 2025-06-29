import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import babel from "vite-plugin-babel";

// https://vitejs.dev/config/
const defaultConfig = defineConfig({
  logLevel: "info",
  plugins: [
    babel({
      babelConfig: {
        plugins: [
          ["@babel/plugin-proposal-decorators", { version: "2023-11" }],
          ["@babel/plugin-proposal-class-static-block", { loose: true }],
          ["@babel/plugin-proposal-class-properties", { loose: true }],
        ],
        parserOpts: {
          plugins: ["decorators", "classProperties", "classStaticBlock"],
        },
      },
    }),
  ],
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
