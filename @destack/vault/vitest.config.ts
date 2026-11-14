import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration(
    {
        tier: "scenario",
        // run every test, each opening a world with a vault, against each database the package stores in
        targets: { database: ["sqlite"] },
        test: {
            include: ["src/**/*.scenario.test.ts"],
            globalSetup: ["src/test/setup.ts"],
        },
    },
    BunEnvironment.system,
);
