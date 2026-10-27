import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

/** The tests running real package managers against a loopback registry, which need npm, bun and deno. */
const INTEGRATION = ["tests/install.test.ts"];

export default defineConfiguration(
    {
        root: fileURLToPath(new URL(".", import.meta.url)),
        test: {
            globalSetup: ["tests/setup.ts"],
            testTimeout: 10_000,
            hookTimeout: 10_000,
            projects: [
                // run the unit tests against each database the package stores in
                {
                    extends: true,
                    targets: { database: ["sqlite", "postgresql"] },
                    test: {
                        include: ["src/**/*.test.ts", "tests/*.test.ts"],
                        exclude: INTEGRATION,
                    },
                },
                // run the package managers' installs once, on sqlite, in a project of their own
                {
                    extends: true,
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
