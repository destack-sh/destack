import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration, SCENARIO_MODULES, scenarioPlugin } from "@destack/test/config";
import { fileURLToPath } from "node:url";

/** The driver of the package's object scenarios, over the notes fixture. */
const NOTES_DRIVER = fileURLToPath(new URL("src/test/driver.ts", import.meta.url));

/** The driver of the package's text scenarios, over one sequence in memory. */
const TEXT_DRIVER = fileURLToPath(new URL("src/test/text.ts", import.meta.url));

export default defineConfiguration(
    {
        plugins: [
            scenarioPlugin([
                { module: NOTES_DRIVER, name: "ObjectDriver" },
                { module: TEXT_DRIVER, name: "TextDriver" },
            ]),
        ],
        // run every test against each database the package stores in
        targets: { database: ["sqlite", "postgresql"] },
        test: {
            projects: [
                // test behaviour in parallel, within tight timeouts
                {
                    extends: true,
                    tier: "unit",
                    test: {
                        name: "unit",
                        include: ["src/**/*.test.ts", "tests/**/*.test.ts", SCENARIO_MODULES],
                    },
                },
            ],
        },
    },
    BunEnvironment.system,
);
