import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration(
    {
        tier: "unit",
        // run every test against the database control mostly runs on, the universe's
        targets: { database: ["postgresql"] },
        test: {
            include: ["src/**/*.test.ts", "tests/**/*.test.ts"],
        },
    },
    BunEnvironment.system,
);
