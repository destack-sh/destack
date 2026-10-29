import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration(
    {
        // run every test against each database the package stores in
        targets: { database: ["sqlite"] },
        test: {
            include: ["src/**/*.test.ts", "tests/**/*.test.ts"],
            testTimeout: 5000,
            hookTimeout: 5000,
            // check the compile-time refusals the type tests state
            typecheck: { enabled: true, include: ["tests/**/*.test-d.ts"] },
        },
    },
    BunEnvironment.system,
);
