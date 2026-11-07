import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

/** The most milliseconds a unit test takes, on the command line's own nouns. */
const UNIT_TIMEOUT = 1000;

/** The most milliseconds a world test takes, with a computer shipping the platform. */
const WORLD_TIMEOUT = 10_000;

/** The tests that run command lines on a computer of a universe. */
const WORLD_TESTS = ["src/**/*.world.test.ts"];

export default defineConfiguration(
    {
        root: fileURLToPath(new URL(".", import.meta.url)),
        ssr: { resolve: { conditions: ["bun"], externalConditions: ["bun"] } },
        // run the tests on sqlite, as machine's computers store
        targets: { database: ["sqlite"] },
        test: {
            projects: [
                // test the command line's nouns on their own
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
                // run command lines on computers, building the platform they ship once per run
                {
                    extends: true,
                    test: {
                        name: "world",
                        include: WORLD_TESTS,
                        globalSetup: ["node_modules/@destack/machine/src/computer/test/setup.ts"],
                        // load the builds a machine places as Bun loads them, so its links answer their kernel imports
                        server: {
                            deps: { external: [/\/destack-machine-[^/]+\/(?:builds|workers)\//u] },
                        },
                        testTimeout: WORLD_TIMEOUT,
                        hookTimeout: WORLD_TIMEOUT,
                    },
                },
            ],
        },
    },
    BunEnvironment.system,
);
