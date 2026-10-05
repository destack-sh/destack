import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

export default defineConfiguration({
    root: fileURLToPath(new URL(".", import.meta.url)),
    test: {
        include: ["src/**/*.test.ts", "tests/*.test.ts"],
        globalSetup: ["tests/setup.ts"],
        testTimeout: 10_000,
        hookTimeout: 10_000,
    },
});
