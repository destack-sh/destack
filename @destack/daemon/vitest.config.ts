import { defineConfig } from "@destack/test/config";
import { fileURLToPath } from "node:url";

export default defineConfig({
    root: fileURLToPath(new URL(".", import.meta.url)),
    test: {
        include: ["src/**/*.test.ts"],
        testTimeout: 5000,
        hookTimeout: 5000,
    },
});
