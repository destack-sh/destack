import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

/** The tests reading exports, rules and meters, which hold no database. */
const UNITS = [
    "src/issue/exception.test.ts",
    "src/otlp/otlp.test.ts",
    "src/inspect/meter.test.ts",
    "src/object/condition.test.ts",
    "src/object/issue.test.ts",
];

/** The most milliseconds a world test takes. */
const WORLD_TIMEOUT = 15_000;

export default defineConfiguration(
    {
        root: fileURLToPath(new URL(".", import.meta.url)),
        test: {
            testTimeout: WORLD_TIMEOUT,
            hookTimeout: WORLD_TIMEOUT,
            projects: [
                // run the units once
                {
                    extends: true,
                    test: { name: "unit", include: UNITS },
                },
                // run every other test against each database the package stores in
                {
                    extends: true,
                    targets: { database: ["sqlite"] },
                    test: {
                        include: ["src/**/*.test.ts"],
                        exclude: UNITS,
                        globalSetup: ["src/test/setup.ts"],
                    },
                },
            ],
        },
    },
    BunEnvironment.system,
);
