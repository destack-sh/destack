import { defineConfig } from "@destack/test/config";

export default defineConfig({
    ssr: { resolve: { conditions: ["bun"], externalConditions: ["bun"] } },
    test: {
        root: import.meta.dirname,
        include: ["src/**/*.test.ts"],
        testTimeout: 10000,
        hookTimeout: 5000,
    },
});
