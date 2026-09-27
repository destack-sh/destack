import { defineConfig } from "@destack/test/config";

export default defineConfig({
    test: { include: ["src/**/*.test.ts"], testTimeout: 5000, hookTimeout: 5000 },
});
