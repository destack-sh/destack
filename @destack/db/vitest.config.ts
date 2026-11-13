import { BunEnvironment } from "@destack/environment/bun";
import { BENCHMARK_MODULES, defineConfiguration, scenarioPlugin } from "@destack/test/config";

/** The tests, without the benchmarks. */
const TESTS = ["src/**/*.test.ts"];

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
        plugins: [scenarioPlugin([{ module: "@destack/db/test", name: "TestDatabaseDriver" }])],
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

                // measure the benchmarks on each database, which `just bench` runs on its own
                {
                    extends: true,
                    tier: "bench",
                    targets: { database: ["sqlite", "postgresql"] },
                    test: { name: "bench", include: [BENCHMARK_MODULES] },
                },
            ],
        },
    },
    BunEnvironment.system,
);
