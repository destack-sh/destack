import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

export default defineConfiguration(
    {
        root: fileURLToPath(new URL(".", import.meta.url)),
        test: {
            projects: [
                // run every test in a real operating system sandbox
                {
                    extends: true,
                    tier: "unit",
                    test: { name: "integration", include: ["src/**/*.test.ts"] },
                },
            ],
        },
    },
    BunEnvironment.system,
);
