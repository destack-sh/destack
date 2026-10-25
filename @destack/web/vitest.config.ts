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
            // store the builds of this package's closure the served applications run on once per run
            globalSetup: ["src/test/setup.ts"],
            // render components in a DOM, and build and serve applications as hosts do
            projects: [
                {
                    extends: false,
                    ...view.defineConfiguration(
                        {
                            root: ROOT,
                            test: {
                                name: "dom",
                                include: ["src/**/*.test.tsx"],
                                testTimeout: 2000,
                            },
                        },
                        environment,
                    ),
                },
                {
                    extends: false,
                    ...defineConfiguration(
                        {
                            root: ROOT,
                            test: {
                                name: "build",
                                include: ["src/**/*.test.ts"],
                                testTimeout: 2000,
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
