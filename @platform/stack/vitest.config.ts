import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration({
    test: {
        root: import.meta.dirname,
        include: ["tests/*.test.ts"],
        testTimeout: 60_000,
        hookTimeout: 10_000,
    },
});
