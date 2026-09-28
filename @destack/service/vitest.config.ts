import { defineConfig } from "@destack/test/config";
import { fileURLToPath } from "node:url";

export default defineConfig({
    root: fileURLToPath(new URL(".", import.meta.url)),
    test: {
        include: ["src/**/*.test.ts"],
        testTimeout: 2000,
        hookTimeout: 2000,
        // collect garbage on demand, as tests of what outlives a collection do
        execArgv: ["--expose-gc"],
    },
});
