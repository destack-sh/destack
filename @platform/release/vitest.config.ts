import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration(
    {
        ssr: { resolve: { conditions: ["bun"], externalConditions: ["bun"] } },
        test: {
            root: import.meta.dirname,
            projects: [
                // run the package's own nouns within tight timeouts
                {
                    extends: true,
                    tier: "unit",
                    test: {
                        name: "unit",
                        include: ["src/**/*.test.ts"],
                        exclude: ["**/*.scenario.test.ts"],
                    },
                },
                // run what drives real processes, such as the vault CLI
                {
                    extends: true,
                    tier: "scenario",
                    test: { name: "scenario", include: ["src/**/*.scenario.test.ts"] },
                },
            ],
        },
    },
    BunEnvironment.system,
);
