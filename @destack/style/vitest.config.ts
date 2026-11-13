import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

import { styleExtension } from "./src/build/index.ts";

/** The package directory. */
const ROOT = fileURLToPath(new URL(".", import.meta.url));

export default defineConfiguration(
    {
        tier: "unit",
        root: ROOT,
        plugins: [
            styleExtension.transform({
                directory: ROOT,
                dependencies: {},
                runtime: "browser",
                server: false,
                environment: BunEnvironment.system,
            }),
        ],
        test: { include: ["src/**/*.test.ts"] },
    },
    BunEnvironment.system,
);
