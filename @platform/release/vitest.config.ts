import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration(
    {
        ssr: { resolve: { conditions: ["bun"], externalConditions: ["bun"] } },
        test: {
            root: import.meta.dirname,
            include: ["src/**/*.test.ts"],
            // load the builds the executable test's universe places as Bun loads them, so its links answer their kernel imports
            server: { deps: { external: [/\/destack-executable-[^/]+\/universe\//u] } },
            testTimeout: 10000,
            hookTimeout: 5000,
        },
    },
    BunEnvironment.system,
);
