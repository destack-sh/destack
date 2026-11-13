import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

export default defineConfiguration(
    {
        tier: "unit",
        root: fileURLToPath(new URL(".", import.meta.url)),
        test: {
            include: ["src/**/*.test.ts"],
            fileParallelism: false,
        },
    },
    BunEnvironment.system,
);
