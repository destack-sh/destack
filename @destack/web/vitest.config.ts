import { fileURLToPath } from "node:url";
import { defineConfiguration } from "@destack/test/config";
import * as view from "@destack/view/test";

/** The package directory, which both projects compile from. */
const ROOT = fileURLToPath(new URL(".", import.meta.url));

export default defineConfiguration({
    test: {
        // render components in a DOM, and build and serve applications as hosts do
        projects: [
            {
                extends: false,
                ...view.defineConfiguration({
                    root: ROOT,
                    test: { name: "dom", include: ["src/**/*.test.tsx"], testTimeout: 2000 },
                }),
            },
            {
                extends: false,
                ...defineConfiguration({
                    root: ROOT,
                    test: { name: "build", include: ["src/**/*.test.ts"], testTimeout: 2000 },
                }),
            },
        ],
    },
});
