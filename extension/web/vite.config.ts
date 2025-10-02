// @ts-nocheck
import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vite";
import babel from "vite-plugin-babel";
import tsconfigPaths from "vite-tsconfig-paths";

// https://vitejs.dev/config/
const defaultConfig = defineConfig({
    logLevel: "info",
    optimizeDeps: { exclude: ["destack"] },
    plugins: [
        babel({
            babelConfig: {
                compact: true,
                presets: [["@babel/preset-typescript", { allowDeclareFields: true }]],
                plugins: [
                    ["@babel/plugin-transform-typescript", { allowDeclareFields: true }],
                    ["@babel/plugin-proposal-decorators", { version: "2023-11" }],
                ],
            },
        }),
        tsconfigPaths({
            projects: ["./tsconfig.json", "../../client/destack_ts/tsconfig.json"],
        }),
    ],
    resolve: {
        preserveSymlinks: true,
        alias: {
            "@destack": fileURLToPath(
                new URL("../../client/destack_ts/typescript", import.meta.url),
            ),
            "@destack-web": fileURLToPath(new URL("./typescript", import.meta.url)),
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
});
export default defaultConfig;
