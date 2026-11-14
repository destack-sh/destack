import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration(
    {
        tier: "scenario",
        // run every test against the database control mostly runs on, the universe's
        targets: { database: ["postgresql"] },
        test: {
            include: ["src/**/*.scenario.test.ts", "tests/**/*.scenario.test.ts"],
        },
    },
    BunEnvironment.system,
);
