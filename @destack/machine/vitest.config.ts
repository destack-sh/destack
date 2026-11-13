import { BunEnvironment } from "@destack/environment/bun";
import { BENCHMARK_MODULES, defineConfiguration, scenarioPlugin } from "@destack/test/config";
import { fileURLToPath } from "node:url";

/** The tests that open a world of a universe, machines and spaces through the space harness. */
const SCENARIO_TESTS = ["src/**/*.scenario.test.ts", "tests/*.scenario.test.ts"];

export default defineConfiguration(
    {
        root: fileURLToPath(new URL(".", import.meta.url)),
        plugins: [
            scenarioPlugin([{ module: "@destack/machine/computer/test", name: "WorldDriver" }]),
        ],
        ssr: { resolve: { conditions: ["bun"], externalConditions: ["bun"] } },
        // run every test against each database the package stores in
        targets: { database: ["sqlite"] },
        test: {
            globalSetup: ["src/computer/test/setup.ts"],
            // load the builds a machine places as Bun loads them, so its links answer their kernel imports
            server: { deps: { external: [/\/destack-machine-[^/]+\/(?:builds|workers)\//u] } },
            projects: [
                // run the tests, as the package's gate does
                {
                    extends: true,
                    tier: "unit",
                    test: {
                        name: "test",
                        include: ["src/**/*.test.ts", "tests/*.test.ts"],
                        exclude: SCENARIO_TESTS,
                    },
                },
                // run the worlds of a universe, machines and spaces, within a world's budget
                {
                    extends: true,
                    tier: "scenario",
                    test: {
                        name: "scenario",
                        include: SCENARIO_TESTS,
                    },
                },
                // measure the product's benchmarks, which `just bench` runs on its own
                {
                    extends: true,
                    tier: "bench",
                    test: { name: "bench", include: [BENCHMARK_MODULES] },
                },
            ],
        },
    },
    BunEnvironment.system,
);
