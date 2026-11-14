import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration(
    {
        ssr: { resolve: { conditions: ["bun"], externalConditions: ["bun"] } },
        test: {
            root: import.meta.dirname,
            // load the builds the executable test's universe and release store place as Bun loads them, so the machine's links answer their builtin imports
            server: {
                deps: {
                    external: [
                        /\/destack-executable-[^/]+\/universe\//u,
                        /\/target\/releases\/(?:trees|layered)\//u,
                    ],
                },
            },
            projects: [
                // run the tests, as the package's gate does
                {
                    extends: true,
                    tier: "unit",
                    test: {
                        name: "unit",
                        include: ["src/**/*.test.ts"],
                        exclude: ["src/**/*.acceptance.test.ts"],
                    },
                },
                // run the release's executables against a loopback universe, which `bun run acceptance` runs on its own
                {
                    extends: true,
                    tier: "bench",
                    test: { name: "acceptance", include: ["src/**/*.acceptance.test.ts"] },
                },
            ],
        },
    },
    BunEnvironment.system,
);
