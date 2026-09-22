import { defineConfig } from "@destack/test/config";
import { fileURLToPath } from "node:url";

export default defineConfig({
    root: fileURLToPath(new URL(".", import.meta.url)),
    test: {
        include: ["tests/*.test.ts", "src/**/*.test.ts"],
        testTimeout: 15000,
        hookTimeout: 15000,
        fileParallelism: true,
        maxWorkers: 2,
        maxConcurrency: 2,
    },
});
