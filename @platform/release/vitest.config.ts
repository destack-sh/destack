import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration(
    {
        ssr: { resolve: { conditions: ["bun"], externalConditions: ["bun"] } },
        test: {
            root: import.meta.dirname,
            include: ["src/**/*.test.ts"],
            // load the builds the executable test's universe and release store place as Bun loads them, so the machine's links answer their builtin imports
            server: {
                deps: {
                    external: [
                        /\/destack-executable-[^/]+\/universe\//u,
                        /\/target\/test\/releases\/(?:trees|layered)\//u,
                    ],
                },
            },
            testTimeout: 10000,
            hookTimeout: 5000,
        },
    },
    BunEnvironment.system,
);
