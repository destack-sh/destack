import { BunEnvironment } from "@destack/environment/bun";
import { BENCHMARK_MODULES, defineConfiguration, scenarioPlugin } from "@destack/test/config";
import { fileURLToPath } from "node:url";

export default defineConfiguration(
    {
        root: fileURLToPath(new URL(".", import.meta.url)),
        plugins: [scenarioPlugin([{ module: "@destack/machine/test", name: "WorldDriver" }])],
        ssr: { resolve: { conditions: ["bun"], externalConditions: ["bun"] } },
        // run every test against each database the package stores in
        targets: { database: ["sqlite"] },
        test: {
            globalSetup: ["src/computer/test/setup.ts"],
            // load the builds a machine places as Bun loads them, so its links answer their kernel imports
            server: { deps: { external: [/\/destack-machine-[^/]+\/(?:builds|workers)\//u] } },
            testTimeout: 5000,
            hookTimeout: 5000,
            projects: [
                // run the tests, as the package's gate does
                {
                    extends: true,
                    test: { name: "test", include: ["src/**/*.test.ts", "tests/*.test.ts"] },
                },
                // measure the product's benchmarks, which `just bench` runs on its own
                { extends: true, test: { name: "bench", include: [BENCHMARK_MODULES] } },
            ],
        },
    },
    BunEnvironment.system,
);
