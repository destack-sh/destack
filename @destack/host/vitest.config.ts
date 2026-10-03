import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

export default defineConfiguration({
    root: fileURLToPath(new URL(".", import.meta.url)),
    test: { include: ["tests/*.test.ts"], testTimeout: 5000, hookTimeout: 5000 },
});
