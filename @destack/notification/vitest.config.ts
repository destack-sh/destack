import { BunEnvironment } from "@destack/environment/bun";
import { fileURLToPath } from "node:url";
import { defineConfiguration, defineProject } from "@destack/test/config";
import * as view from "@destack/view/test";

/** The host the tests build and run on. */
const environment = BunEnvironment.system;

/** The most milliseconds a server test serving a world's spaces takes. */
const WORLD_TIMEOUT = 15_000;

export default defineConfiguration(
    {
        test: {
            // keep each project's own plugins, which it lists without the declaring config's
            projects: [
                {
                    extends: false,
                    // run the server tests against each database the package stores in
                    targets: { database: ["sqlite", "postgresql"] },
                    ...defineProject(
                        {
                            test: {
                                name: "server",
                                include: ["tests/*.test.ts"],
                                globalSetup: ["src/test/setup.ts"],
                                testTimeout: WORLD_TIMEOUT,
                                hookTimeout: WORLD_TIMEOUT,
                            },
                        },
                        environment,
                    ),
                },
                {
                    extends: false,
                    // render the views over the database a browser stores in
                    targets: { database: ["sqlite"] },
                    ...view.defineConfiguration(
                        {
                            root: fileURLToPath(new URL(".", import.meta.url)),
                            test: {
                                name: "view",
                                include: ["tests/view/**/*.test.tsx"],
                                globalSetup: ["src/test/setup.ts"],
                                testTimeout: 5000,
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
