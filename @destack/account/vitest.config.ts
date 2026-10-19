import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration({
    test: {
        include: ["src/**/*.test.ts", "tests/**/*.test.ts"],
        globalSetup: ["src/test/setup.ts"],
        testTimeout: 10000,
        hookTimeout: 10000,
    },
});
