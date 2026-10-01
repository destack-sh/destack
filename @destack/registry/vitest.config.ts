import { defineConfig } from "@destack/test/config";
import { fileURLToPath } from "node:url";

export default defineConfig({
    root: fileURLToPath(new URL(".", import.meta.url)),
    test: {
        include: ["tests/*.test.ts"],
        globalSetup: ["tests/setup.ts"],
        testTimeout: 10_000,
        hookTimeout: 10_000,
    },
});
