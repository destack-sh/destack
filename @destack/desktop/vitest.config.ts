import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

export default defineConfiguration(
    {
        root: fileURLToPath(new URL(".", import.meta.url)),
        ssr: { resolve: { conditions: ["bun"], externalConditions: ["bun"] } },
        // run the world tests on sqlite, as machine's computers store
        targets: { database: ["sqlite"] },
        test: {
            include: ["src/**/*.test.ts"],
            // build the platform a test computer ships once per run, as machine's tests do
            globalSetup: ["node_modules/@destack/machine/src/computer/test/setup.ts"],
            // load the builds a machine places as Bun loads them, so its links answer their kernel imports
            server: { deps: { external: [/\/destack-machine-[^/]+\/(?:builds|workers)\//u] } },
            testTimeout: 10000,
            env: { DESTACK_STAGE: "production" },
        },
    },
    BunEnvironment.system,
);
