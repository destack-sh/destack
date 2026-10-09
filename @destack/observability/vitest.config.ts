import { fileURLToPath } from "node:url";
import { defineConfiguration, defineProject } from "@destack/test/config";
import * as view from "@destack/view/test";

export default defineConfiguration({
    test: {
        // keep each project's own plugins, which it lists without the declaring config's
        projects: [
            {
                extends: false,
                ...defineProject({
                    test: {
                        name:
                            process.env["DESTACK_TEST_POSTGRES"] === undefined
                                ? "sqlite"
                                : "postgresql",
                        include: ["src/**/*.test.ts", "tests/*.test.ts"],
                        testTimeout: 5000,
                        hookTimeout: 5000,
                    },
                }),
            },
            {
                extends: false,
                ...view.defineConfiguration({
                    root: fileURLToPath(new URL(".", import.meta.url)),
                    test: {
                        name: "view",
                        include: ["tests/view/**/*.test.tsx"],
                        testTimeout: 5000,
                    },
                }),
            },
        ],
    },
});
