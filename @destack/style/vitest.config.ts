import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

import { styleExtension } from "./src/build/index.ts";

/** The package directory. */
const ROOT = fileURLToPath(new URL(".", import.meta.url));

export default defineConfiguration(
    {
        root: ROOT,
        plugins: [
            styleExtension.transform({
                directory: ROOT,
                dependencies: {},
                runtime: "browser",
                server: false,
                options: {},
                environment: BunEnvironment.system,
            }),
        ],
        test: { include: ["src/**/*.test.ts"], testTimeout: 3000, hookTimeout: 3000 },
    },
    BunEnvironment.system,
);
