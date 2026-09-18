import { defineConfig } from "@destack/test/config";

export default defineConfig({
    test: { include: ["src/**/*.test.ts"], testTimeout: 1000, hookTimeout: 1000 },
});
