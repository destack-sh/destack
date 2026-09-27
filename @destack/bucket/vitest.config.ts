import { defineConfig } from "@destack/test/config";
import { fileURLToPath } from "node:url";

export default defineConfig({
    root: fileURLToPath(new URL(".", import.meta.url)),
    test: {
        include: ["src/**/*.test.ts", "tests/*.test.ts"],
        testTimeout: 10000,
        hookTimeout: 10000,
        fileParallelism: false,
    },
});
