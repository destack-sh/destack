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
                        include: ["src/**/*.test.ts"],
                        exclude: ["src/**/*.scenario.test.ts"],
                    },
                },
                // run the tests forking a space against each database the package stores in
                {
                    extends: true,
                    tier: "scenario",
                    targets: { database: ["sqlite"] },
                    test: {
                        include: ["src/**/*.scenario.test.ts"],
                        globalSetup: ["src/test/setup.ts"],
                        fileParallelism: false,
                        sequence: { groupOrder: 1 },
                    },
                },
            ],
        },
    },
    BunEnvironment.system,
);
