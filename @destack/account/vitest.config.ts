import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration(
    {
        // run every test against the universe's database, where accounts live
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
