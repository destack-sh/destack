import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration(
    {
        tier: "unit",
        // run every test against each database the package stores in
        targets: { database: ["sqlite"] },
        test: {
            include: ["src/**/*.test.ts"],
            globalSetup: ["src/test/setup.ts"],
        },
    },
    BunEnvironment.system,
);
