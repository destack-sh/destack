import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration({
    test: { include: ["src/**/*.test.ts"], testTimeout: 5000, hookTimeout: 5000 },
});
