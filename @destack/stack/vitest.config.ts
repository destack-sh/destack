import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

/** The most milliseconds a unit test takes, on one package's tables. */
const UNIT_TIMEOUT = 5000;

/** The most milliseconds a world test takes. */
const WORLD_TIMEOUT = 15_000;

/** The tests that open a world of a universe, machines and spaces. */
const WORLD_TESTS = ["src/**/*.world.test.ts", "tests/*.test.ts"];

export default defineConfiguration(
    {
        root: fileURLToPath(new URL(".", import.meta.url)),
        // run every test against each database the package stores in
        targets: { database: ["sqlite"] },
        test: {
            globalSetup: ["src/test/setup.ts"],
            projects: [
                // test the package's nouns on their own tables, within tight timeouts
                {
                    extends: true,
                    test: {
                        name: "unit",
                        include: ["src/**/*.test.ts"],
                        exclude: WORLD_TESTS,
                        testTimeout: UNIT_TIMEOUT,
                        hookTimeout: UNIT_TIMEOUT,
                    },
                },
                // run whole worlds of a universe, machines and spaces
                {
                    extends: true,
                    test: {
                        name: "world",
                        include: WORLD_TESTS,
                        testTimeout: WORLD_TIMEOUT,
                        hookTimeout: WORLD_TIMEOUT,
                    },
                },
            ],
        },
    },
    BunEnvironment.system,
);
