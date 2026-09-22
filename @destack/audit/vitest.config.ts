import { defineConfig } from "@destack/test/config";

export default defineConfig({
    test: {
        name: process.env.DESTACK_TEST_POSTGRES ? "postgresql" : "turso",
        include: ["tests/**/*.test.ts"],
        testTimeout: 5000,
        hookTimeout: 5000,
    },
});
