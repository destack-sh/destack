import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

export default defineConfiguration(
    {
        root: fileURLToPath(new URL(".", import.meta.url)),
        test: {
            globalSetup: ["tests/setup.ts"],
            projects: [
                // run the package's own nouns against each database the package stores in
                {
                    extends: true,
                    tier: "unit",
                    targets: { database: ["sqlite"] },
                    test: {
                        name: "unit",
                        include: ["src/**/*.test.ts", "tests/*.test.ts"],
                        exclude: ["tests/*.scenario.test.ts"],
                    },
                },
                // open a forge on a machine and publish, follow and serve through it
                {
                    extends: true,
                    tier: "scenario",
                    targets: { database: ["sqlite"] },
                    test: { name: "scenario", include: ["tests/*.scenario.test.ts"] },
                },
            ],
        },
    },
    BunEnvironment.system,
);
