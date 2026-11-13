import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration(
    {
        tier: "unit",
        test: {
            include: ["src/**/*.test.ts", "tests/**/*.test.ts"],
        },
    },
    BunEnvironment.system,
);
