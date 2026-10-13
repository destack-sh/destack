import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

export default defineConfiguration({
    root: fileURLToPath(new URL(".", import.meta.url)),
    test: {
        include: ["src/**/*.test.ts"],
        testTimeout: 2000,
        hookTimeout: 2000,
        // expose garbage collection to tests of what outlives a collection
        execArgv: ["--expose-gc"],
    },
});
