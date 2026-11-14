import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration, SCENARIO_MODULES, scenarioPlugin } from "@destack/test/config";
import { fileURLToPath } from "node:url";

export default defineConfiguration(
    {
        root: fileURLToPath(new URL(".", import.meta.url)),
        plugins: [scenarioPlugin([{ module: "@destack/sync/test", name: "FeedDriver" }])],
        // run every test against each database the package stores in
        targets: { database: ["sqlite", "postgresql"] },
        test: {
            projects: [
                // test behaviour in parallel, within tight timeouts
                {
                    extends: true,
                    tier: "unit",
                    test: { name: "unit", include: ["src/**/*.test.ts", SCENARIO_MODULES] },
                },
            ],
        },
    },
    BunEnvironment.system,
);
