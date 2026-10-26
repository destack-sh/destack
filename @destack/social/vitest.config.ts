import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

/** The most milliseconds a test takes, each serving a world's space: 3 s alone on Postgres, under 4 workers more. */
const WORLD_TIMEOUT = 15_000;

export default defineConfiguration(
    {
        // run every test against each database the package stores in
        targets: { database: ["sqlite", "postgresql"] },
        test: {
            include: ["src/**/*.test.ts"],
            globalSetup: ["src/test/setup.ts"],
            testTimeout: WORLD_TIMEOUT,
            hookTimeout: WORLD_TIMEOUT,
        },
    },
    BunEnvironment.system,
);
