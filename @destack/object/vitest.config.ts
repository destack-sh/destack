import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration({
    // run every test against each database the package stores in
    targets: { database: ["sqlite", "postgresql"] },
    test: {
        include: ["src/**/*.test.ts", "tests/**/*.test.ts"],
        testTimeout: 5000,
        hookTimeout: 5000,
    },
});
