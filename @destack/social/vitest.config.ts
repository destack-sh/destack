import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

/** The most milliseconds a test serving a world's space takes. */
const WORLD_TIMEOUT = 15_000;

export default defineConfiguration(
    {
        // run every test against each database the package stores in
        targets: { database: ["sqlite"] },
        test: {
            include: ["src/**/*.test.ts"],
            globalSetup: ["src/test/setup.ts"],
            testTimeout: WORLD_TIMEOUT,
            hookTimeout: WORLD_TIMEOUT,
        },
    },
    BunEnvironment.system,
);
