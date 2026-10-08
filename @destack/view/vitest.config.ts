import { fileURLToPath } from "node:url";
import { defineConfiguration } from "@destack/test/config";
import * as view from "./src/test/index.ts";

/** The package directory, which every project compiles from. */
const ROOT = fileURLToPath(new URL(".", import.meta.url));

export default defineConfiguration({
    test: {
        // render components in a DOM and on the server, and build packages as hosts do
        projects: [
            {
                extends: false,
                ...view.defineConfiguration({
                    root: ROOT,
                    test: {
                        name: "dom",
                        include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
                        exclude: [
                            "src/build/**",
                            "src/**/*.server.test.ts",
                            "src/**/*.hydration.test.tsx",
                        ],
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
                        include: ["src/**/*.server.test.ts"],
                        testTimeout: 2000,
                    },
                }),
            },
            {
                extends: false,
                ...view.defineConfiguration(
                    {
                        root: ROOT,
                        test: {
                            name: "hydration",
                            include: ["src/**/*.hydration.test.tsx"],
                            testTimeout: 5000,
                        },
                    },
                    { isHydrated: true },
                ),
            },
            {
                extends: false,
                ...defineConfiguration({
                    root: ROOT,
                    test: { name: "build", include: ["src/build/**/*.test.ts"], testTimeout: 2000 },
                }),
            },
        ],
    },
});
