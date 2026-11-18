import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration(
    {
        test: {
            projects: [
                // run the package's own nouns within tight timeouts
                {
                    extends: true,
                    tier: "unit",
                    test: {
                        name: "unit",
                        include: ["src/**/*.test.ts", "tests/**/*.test.ts"],
                        exclude: ["**/*.scenario.test.ts"],
                    },
                },
                // run the tests that outlast a real host's own timeouts
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
