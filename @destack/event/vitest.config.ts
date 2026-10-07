import { defineConfiguration, defineProject } from "@destack/test/config";

export default defineConfiguration({
    test: {
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
        ],
    },
});
