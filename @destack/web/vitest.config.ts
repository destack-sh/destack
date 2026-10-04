import { fileURLToPath } from "node:url";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration({
    root: fileURLToPath(new URL(".", import.meta.url)),
    test: { include: ["src/**/*.test.ts"], testTimeout: 2000 },
});
