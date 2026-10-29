import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration(
    {
        // run every test against each database the package stores in
        targets: { database: ["sqlite"] },
        test: {
            include: ["src/**/*.test.ts"],
            globalSetup: ["src/test/setup.ts"],
            testTimeout: 5000,
            hookTimeout: 5000,
        },
    },
    BunEnvironment.system,
);
