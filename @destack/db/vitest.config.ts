import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

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
        test: {
            include: ["src/**/*.test.ts"],
            globalSetup: ["src/test/setup.ts"],
            testTimeout: 5000,
            hookTimeout: 5000,
            projects: [
                // run the tests of every database on SQLite
                {
                    extends: true,
                    targets: { database: ["sqlite"] },
                    test: { exclude: POSTGRESQL },
                },
                // run every other test on PostgreSQL, its own behavior included
                {
                    extends: true,
                    targets: { database: ["postgresql"] },
                    test: { exclude: SQLITE },
                },
            ],
        },
    },
    BunEnvironment.system,
);
