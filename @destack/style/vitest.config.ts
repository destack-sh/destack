import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

import { styleExtension } from "./src/build/index.ts";

/** The package directory. */
const ROOT = fileURLToPath(new URL(".", import.meta.url));

export default defineConfiguration({
    root: ROOT,
    plugins: [
        styleExtension.transform({
            directory: ROOT,
            runtime: "browser",
            server: false,
            options: {},
        }),
    ],
    test: { include: ["src/**/*.test.ts"], testTimeout: 3000, hookTimeout: 3000 },
});
