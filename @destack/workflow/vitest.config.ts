import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration(
    {
        tier: "unit",
        // run every test against each database the package stores in
        targets: { database: ["sqlite"] },
        test: {
            include: ["src/**/*.test.ts", "tests/**/*.test.ts"],
            // check the compile-time refusals the type tests state
            typecheck: { enabled: true, include: ["tests/**/*.test-d.ts"] },
        },
    },
    BunEnvironment.system,
);
