import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

/** The tests that run command lines on a computer of a universe. */
const SCENARIO_TESTS = ["src/**/*.scenario.test.ts"];

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
                    tier: "unit",
                    test: {
                        name: "unit",
                        include: ["src/**/*.test.ts"],
                        exclude: SCENARIO_TESTS,
                    },
                },
                // run command lines on computers, building the platform they ship once per run
                {
                    extends: true,
                    tier: "scenario",
                    test: {
                        name: "scenario",
                        include: SCENARIO_TESTS,
                        globalSetup: ["node_modules/@destack/machine/src/computer/test/setup.ts"],
                        // load the builds a machine places as Bun loads them, so its links answer their kernel imports
                        server: {
                            deps: { external: [/\/destack-machine-[^/]+\/(?:builds|workers)\//u] },
                        },
                    },
                },
            ],
        },
    },
    BunEnvironment.system,
);
