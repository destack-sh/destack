import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration, SCENARIO_MODULES, scenarioPlugin } from "@destack/test/config";

/** The tests and the scenarios. */
const TESTS = ["src/**/*.test.ts", SCENARIO_MODULES];

/** The tests of PostgreSQL's connector, host, channel and concurrent commits. */
const POSTGRESQL = ["src/postgres/**/*.test.ts", "src/log/test/concurrency.test.ts"];

/** The tests of the SQLite hosts, which open their own SQLite whatever the target. */
const SQLITE = [
    "src/sqlite/**/*.test.ts",
    "src/bun/**/*.test.ts",
    "src/cloudflare/**/*.test.ts",
    "src/browser/**/*.test.ts",
];

export default defineConfiguration(
    {
        plugins: [scenarioPlugin([{ module: "@destack/db/test", name: "DatabaseDriver" }])],
        test: {
            globalSetup: ["src/test/setup.ts"],
            projects: [
                // run the tests of every database on SQLite
                {
                    extends: true,
                    tier: "unit",
                    targets: { database: ["sqlite"] },
                    test: { include: TESTS, exclude: POSTGRESQL },
                },
                // run every other test on PostgreSQL, its own behavior included
                {
                    extends: true,
                    tier: "unit",
                    targets: { database: ["postgresql"] },
                    test: { include: TESTS, exclude: SQLITE },
                },
            ],
        },
    },
    BunEnvironment.system,
);
