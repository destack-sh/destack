import { fileURLToPath } from "node:url";
import { defineConfiguration } from "./src/test/index.ts";

export default defineConfiguration({
    root: fileURLToPath(new URL(".", import.meta.url)),
    test: { include: ["src/**/*.test.ts", "src/**/*.test.tsx"], testTimeout: 2000 },
});
