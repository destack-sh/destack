import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

/** The tests installing with Bun against a loopback registry. */
const INTEGRATION = ["tests/install.test.ts"];

export default defineConfiguration(
    {
        root: fileURLToPath(new URL(".", import.meta.url)),
        test: {
            globalSetup: ["tests/setup.ts"],
            projects: [
                // run the unit tests against each database the package stores in
                {
                    extends: true,
                    tier: "unit",
                    targets: { database: ["sqlite"] },
                    test: {
                        include: ["src/**/*.test.ts", "tests/*.test.ts"],
                        exclude: INTEGRATION,
                    },
                },
                // run Bun's installs once, on sqlite, in a project of their own
                {
                    extends: true,
                    tier: "scenario",
                    test: {
                        name: "integration",
                        include: INTEGRATION,
                        provide: { targets: { database: "sqlite" } },
                    },
                },
            ],
        },
    },
    BunEnvironment.system,
);
