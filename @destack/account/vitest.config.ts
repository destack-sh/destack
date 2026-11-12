import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration(
    {
        // run every test against the database accounts mostly run on, the universe's
        targets: { database: ["postgresql"] },
        test: {
            include: ["src/**/*.test.ts", "tests/**/*.test.ts"],
            globalSetup: ["src/test/setup.ts"],
            testTimeout: 10000,
            hookTimeout: 10000,
        },
    },
    BunEnvironment.system,
);
