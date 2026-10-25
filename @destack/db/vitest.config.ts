import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

/** The tests of PostgreSQL's own behavior: its connector, its host, its channel and its concurrent commits. */
const POSTGRESQL = ["src/postgres/**/*.test.ts", "src/log/test/concurrency.test.ts"];

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
                // run every test on PostgreSQL, its own behavior included
                {
                    extends: true,
                    targets: { database: ["postgresql"] },
                },
            ],
        },
    },
    BunEnvironment.system,
);
