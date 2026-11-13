import { BunEnvironment } from "@destack/environment/bun";
import { fileURLToPath } from "node:url";
import { defineConfiguration } from "@destack/test/config";
import * as view from "@destack/view/test";

/** The host the tests build and run on. */
const environment = BunEnvironment.system;

/** The package directory, which every project compiles from. */
const ROOT = fileURLToPath(new URL(".", import.meta.url));

export default defineConfiguration(
    {
        test: {
            // render components in a DOM and on the server, and build packages as hosts do
            projects: [
                {
                    extends: false,
                    tier: "unit",
                    // render the views over the database a browser stores in
                    targets: { database: ["sqlite"] },
                    ...view.defineConfiguration(
                        {
                            tier: "unit",
                            root: ROOT,
                            test: {
                                name: "dom",
                                include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
                                exclude: [
                                    "src/build/**",
                                    "src/**/*.server.test.ts",
                                    "src/**/*.hydration.test.tsx",
                                ],
                            },
                        },
                        environment,
                    ),
                },
                {
                    extends: false,
                    tier: "unit",
                    ...view.defineConfiguration(
                        {
                            tier: "unit",
                            root: ROOT,
                            test: {
                                name: "server",
                                environment: "node",
                                include: ["src/**/*.server.test.ts"],
                            },
                        },
                        environment,
                    ),
                },
                {
                    extends: false,
                    tier: "unit",
                    ...view.defineConfiguration(
                        {
                            tier: "unit",
                            root: ROOT,
                            test: {
                                name: "hydration",
                                include: ["src/**/*.hydration.test.tsx"],
                            },
                        },
                        environment,
                        { isHydrated: true },
                    ),
                },
                {
                    extends: false,
                    tier: "scenario",
                    ...defineConfiguration(
                        {
                            tier: "scenario",
                            root: ROOT,
                            test: {
                                name: "build",
                                include: ["src/build/**/*.test.ts"],
                                // store the notebook package's dependency builds once per run
                                globalSetup: ["src/test/setup.ts"],
                            },
                        },
                        environment,
                    ),
                },
            ],
        },
    },
    environment,
);
