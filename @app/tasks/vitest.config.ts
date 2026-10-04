import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration({
    test: { include: ["tests/**/*.test.ts"], testTimeout: 5000, hookTimeout: 5000 },
});
