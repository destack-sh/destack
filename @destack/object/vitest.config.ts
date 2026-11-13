import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration(
    {
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
                        include: ["src/**/*.test.ts", "tests/**/*.test.ts"],
                        exclude: ["**/*.bench.test.ts"],
                    },
                },
                // run performance budgets one file at a time after the unit tests, keeping their measurements apart
                {
                    extends: true,
                    tier: "bench",
                    test: {
                        name: "bench",
                        include: ["tests/**/*.bench.test.ts"],
                        fileParallelism: false,
                        sequence: { groupOrder: 1 },
                    },
                },
            ],
        },
    },
    BunEnvironment.system,
);
