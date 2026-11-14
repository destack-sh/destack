import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration(
    {
        tier: "unit",
        ssr: { resolve: { conditions: ["bun"], externalConditions: ["bun"] } },
        test: {
            root: import.meta.dirname,
            include: ["src/**/*.test.ts"],
        },
    },
    BunEnvironment.system,
);
