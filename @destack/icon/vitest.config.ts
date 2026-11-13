import { BunEnvironment } from "@destack/environment/bun";
import { fileURLToPath } from "node:url";
import { defineConfiguration } from "@destack/view/test";

export default defineConfiguration(
    {
        tier: "unit",
        root: fileURLToPath(new URL(".", import.meta.url)),
        test: { include: ["src/**/*.test.{ts,tsx}"] },
    },
    BunEnvironment.system,
);
