import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

export default defineConfiguration(
    {
        root: fileURLToPath(new URL(".", import.meta.url)),
        test: {
            projects: [
                // run the tests that fork no image once
                {
                    extends: true,
                    tier: "unit",
                    test: {
                        name: "unit",
                        include: ["tests/**/*.test.ts"],
                        exclude: ["tests/**/*.scenario.test.ts"],
                    },
                },
                // run the tests forking the home against each database the package stores in
                {
                    extends: true,
                    tier: "scenario",
                    targets: { database: ["sqlite"] },
                    test: {
                        include: ["tests/**/*.scenario.test.ts"],
                        globalSetup: ["src/test/setup.ts"],
                    },
                },
            ],
        },
    },
    BunEnvironment.system,
);
