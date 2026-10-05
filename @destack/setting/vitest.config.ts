import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration({
    test: {
        include: ["src/**/*.test.ts", "tests/**/*.test.ts"],
        testTimeout: 1000,
        hookTimeout: 1000,
    },
});
