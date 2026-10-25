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
            // render in a DOM, and render to a string as a server does
            projects: [
                {
                    extends: false,
                    ...view.defineConfiguration(
                        {
                            root: ROOT,
                            test: {
                                name: "dom",
                                include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
                                testTimeout: 2000,
                            },
                        },
                        environment,
                    ),
                },
                {
                    extends: false,
                    ...view.defineConfiguration(
                        {
                            root: ROOT,
                            test: {
                                name: "server",
                                environment: "node",
                                include: ["tests/**/*.test.tsx"],
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
