import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration, defineProject } from "@destack/test/config";

export default defineConfiguration(
    {
        test: {
            projects: [
                {
                    extends: false,
                    tier: "unit",
                    // resolve and describe settings once, as no database stores them
                    ...defineProject(
                        {
                            test: {
                                name: "setting",
                                include: ["tests/setting.test.ts"],
                            },
                        },
                        BunEnvironment.system,
                    ),
                },
                {
                    extends: false,
                    tier: "unit",
                    // store values against both databases, as the universe and spaces keep settings
                    targets: { database: ["sqlite", "postgresql"] },
                    ...defineProject(
                        {
                            test: {
                                name: "store",
                                include: ["tests/**/*.test.ts"],
                                exclude: ["tests/setting.test.ts"],
                            },
                        },
                        BunEnvironment.system,
                    ),
                },
            ],
        },
    },
    BunEnvironment.system,
);
