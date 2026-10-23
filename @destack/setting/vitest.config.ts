import { defineConfiguration, defineProject } from "@destack/test/config";

export default defineConfiguration({
    test: {
        projects: [
            {
                extends: false,
                // resolve and describe settings once, as no database stores them
                ...defineProject({
                    test: {
                        name: "setting",
                        include: ["tests/setting.test.ts"],
                        testTimeout: 1000,
                        hookTimeout: 1000,
                    },
                }),
            },
            {
                extends: false,
                // store values against each database the package stores in
                targets: { database: ["sqlite", "postgresql"] },
                ...defineProject({
                    test: {
                        name: "store",
                        include: ["tests/**/*.test.ts"],
                        exclude: ["tests/setting.test.ts"],
                        testTimeout: 1000,
                        hookTimeout: 1000,
                    },
                }),
            },
        ],
    },
});
