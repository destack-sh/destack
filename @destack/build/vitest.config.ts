import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

export default defineConfiguration(
    {
        tier: "unit",
        root: fileURLToPath(new URL(".", import.meta.url)),
        test: {
            include: ["tests/*.test.ts", "src/**/*.test.ts"],
            fileParallelism: true,
            maxWorkers: 2,
            maxConcurrency: 2,
        },
    },
    BunEnvironment.system,
);
