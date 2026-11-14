import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

export default defineConfiguration(
    {
        root: fileURLToPath(new URL(".", import.meta.url)),
        test: {
            fileParallelism: true,
            maxWorkers: 2,
            maxConcurrency: 2,
            projects: [
                // run the package's own nouns
                {
                    extends: true,
                    tier: "unit",
                    test: { name: "unit", include: ["src/**/*.test.ts"] },
                },
                // build, check and evaluate packages with the real compiler
                {
                    extends: true,
                    tier: "scenario",
                    test: { name: "scenario", include: ["tests/*.scenario.test.ts"] },
                },
            ],
        },
    },
    BunEnvironment.system,
);
