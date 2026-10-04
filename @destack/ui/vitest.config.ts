import { fileURLToPath } from "node:url";
import { defineConfiguration } from "@destack/test/config";
import * as view from "@destack/view/test";

/** The package directory, which both projects compile from. */
const ROOT = fileURLToPath(new URL(".", import.meta.url));

export default defineConfiguration({
    test: {
        // render in a DOM, and render to a string as a server does
        projects: [
            {
                extends: false,
                ...view.defineConfiguration({
                    root: ROOT,
                    test: {
                        name: "dom",
                        include: ["src/**/*.test.tsx"],
                        testTimeout: 2000,
                    },
                }),
            },
            {
                extends: false,
                ...view.defineConfiguration({
                    root: ROOT,
                    test: {
                        name: "server",
                        environment: "node",
                        include: ["tests/**/*.test.tsx"],
                        testTimeout: 2000,
                    },
                }),
            },
        ],
    },
});
