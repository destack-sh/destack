import { fileURLToPath } from "node:url";
import { defineConfiguration } from "@destack/view/test";

export default defineConfiguration({
    root: fileURLToPath(new URL(".", import.meta.url)),
    test: { include: ["src/**/*.test.tsx"], testTimeout: 2000 },
});
