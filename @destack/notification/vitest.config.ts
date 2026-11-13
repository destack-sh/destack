import { BunEnvironment } from "@destack/environment/bun";
import { fileURLToPath } from "node:url";
import { defineConfiguration, defineProject } from "@destack/test/config";
import * as view from "@destack/view/test";

/** The host the tests build and run on. */
const environment = BunEnvironment.system;

export default defineConfiguration(
    {
        test: {
            // keep each project's own plugins, which it lists without the declaring config's
            projects: [
                {
                    extends: false,
                    tier: "scenario",
                    // run the server tests against the database inboxes mostly run on, the spaces'
                    targets: { database: ["sqlite"] },
                    ...defineProject(
                        {
                            test: {
                                name: "server",
                                include: ["tests/*.test.ts"],
                                globalSetup: ["src/test/setup.ts"],
                            },
                        },
                        environment,
                    ),
                },
                {
                    extends: false,
                    tier: "unit",
                    // render the views over the database a browser stores in
                    targets: { database: ["sqlite"] },
                    ...view.defineConfiguration(
                        {
                            tier: "unit",
                            root: fileURLToPath(new URL(".", import.meta.url)),
                            test: {
                                name: "view",
                                include: ["tests/view/**/*.test.tsx"],
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
