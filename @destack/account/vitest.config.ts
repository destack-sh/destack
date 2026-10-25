import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration(
    {
        // run every test against each database the package stores in
        targets: { database: ["sqlite", "postgresql"] },
        test: {
            include: ["src/**/*.test.ts", "tests/**/*.test.ts"],
            globalSetup: ["src/test/setup.ts"],
            testTimeout: 10000,
            hookTimeout: 10000,
        },
    },
    BunEnvironment.system,
);
