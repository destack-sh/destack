import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration(
    {
        test: { include: ["src/**/*.test.ts", "tests/**/*.test.ts"], testTimeout: 10_000 },
    },
    BunEnvironment.system,
);
