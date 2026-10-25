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
                    test: {
                        name: "unit",
                        include: ["src/**/*.test.ts"],
                        exclude: ["src/**/*.bench.test.ts"],
                        testTimeout: 5000,
                        hookTimeout: 5000,
                    },
                },
                // run performance budgets one file at a time after the unit tests, keeping their measurements apart
                {
                    extends: true,
                    test: {
                        name: "bench",
                        include: ["src/**/*.bench.test.ts"],
                        fileParallelism: false,
                        sequence: { groupOrder: 1 },
                        testTimeout: 30_000,
                        hookTimeout: 30_000,
                    },
                },
            ],
        },
    },
    BunEnvironment.system,
);
