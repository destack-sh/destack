import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

export default defineConfiguration(
    {
        root: fileURLToPath(new URL(".", import.meta.url)),
        tier: "unit",
        test: { include: ["src/**/*.test.ts"] },
    },
    BunEnvironment.system,
);
