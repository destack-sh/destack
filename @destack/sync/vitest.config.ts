import { defineConfig } from "@destack/test/config";
import { fileURLToPath } from "node:url";

export default defineConfig({
    root: fileURLToPath(new URL(".", import.meta.url)),
    test: {
        projects: [
            // test behaviour in parallel, within tight timeouts
            {
                extends: true,
                test: {
                    name: "unit",
                    include: ["src/**/*.test.ts"],
                    exclude: ["src/**/*.bench.test.ts"],
                    testTimeout: 5000,
                    hookTimeout: 5000,
                },
            },
            // hold performance budgets one file at a time, so that measurements do not contend
            {
                extends: true,
                test: {
                    name: "bench",
                    include: ["src/**/*.bench.test.ts"],
                    fileParallelism: false,
                    testTimeout: 30_000,
                    hookTimeout: 30_000,
                },
            },
        ],
    },
});
