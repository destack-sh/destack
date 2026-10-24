import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration(
    {
        test: { include: ["src/**/*.test.ts"], testTimeout: 2000 },
    },
    BunEnvironment.system,
);
