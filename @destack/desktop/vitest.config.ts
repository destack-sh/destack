import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

/** The most milliseconds a unit test takes, on the desktop's own objects. */
const UNIT_TIMEOUT = 5000;

/** The most milliseconds a world test takes, with computers shipping the platform. */
const WORLD_TIMEOUT = 10_000;

/** The tests that open a world of a universe and computers. */
const WORLD_TESTS = ["src/**/*.world.test.ts"];

export default defineConfiguration(
    {
        root: fileURLToPath(new URL(".", import.meta.url)),
        ssr: { resolve: { conditions: ["bun"], externalConditions: ["bun"] } },
        // run the tests on sqlite, as machine's computers store
        targets: { database: ["sqlite"] },
        test: {
            env: { DESTACK_STAGE: "production" },
            projects: [
                // test the desktop's nouns on their own objects
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
                // run worlds of computers, building the platform they ship once per run
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
