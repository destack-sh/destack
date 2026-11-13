import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

export default defineConfiguration(
    {
        root: fileURLToPath(new URL(".", import.meta.url)),
        // run every test against each database the package stores in
        targets: { database: ["sqlite", "postgresql"] },
        test: {
            projects: [
                // test behaviour in parallel, within tight timeouts
                {
                    extends: true,
                    tier: "unit",
                    test: {
                        name: "unit",
                        include: ["src/**/*.test.ts"],
                        exclude: ["src/**/*.bench.test.ts"],
                    },
                },
                // run performance budgets one file at a time after the unit tests, keeping their measurements apart
                {
                    extends: true,
                    tier: "bench",
                    test: {
                        name: "bench",
                        include: ["src/**/*.bench.test.ts"],
                        fileParallelism: false,
                        sequence: { groupOrder: 1 },
                    },
                },
            ],
        },
    },
    BunEnvironment.system,
);
