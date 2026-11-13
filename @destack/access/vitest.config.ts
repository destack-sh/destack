import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration(
    {
        tier: "unit",
        // run every test against each database the package stores in
        targets: { database: ["sqlite", "postgresql"] },
        test: { include: ["src/**/*.test.ts"] },
    },
    BunEnvironment.system,
);
