import { expect, test } from "vitest";
import { defineConfiguration } from "./config.ts";

test("run each project once per combination of its targets, named after them, with the root's setup in each extending project", () => {
    // declare a root matrix, a project with targets and a colored label, and one extending the root
    const configuration = defineConfiguration({
        targets: { database: ["sqlite", "postgresql"] },
        test: {
            globalSetup: ["setup.ts"],
            projects: [
                {
                    extends: false,
                    targets: { database: ["sqlite"], runtime: ["bun", "node"] },
                    test: { name: { label: "unit", color: "green" }, include: ["src/**"] },
                },
                { extends: true, test: { name: "simulation" } },
            ],
        },
    });

    // expect one project per combination with its values, and the root's setup in extending ones
    expect(configuration.test?.projects).toEqual([
        {
            extends: false,
            test: {
                name: { label: "unit:sqlite,bun", color: "green" },
                include: ["src/**"],
                provide: { targets: { database: "sqlite", runtime: "bun" } },
            },
        },
        {
            extends: false,
            test: {
                name: { label: "unit:sqlite,node", color: "green" },
                include: ["src/**"],
                provide: { targets: { database: "sqlite", runtime: "node" } },
            },
        },
        {
            extends: true,
            test: {
                globalSetup: ["setup.ts"],
                name: "simulation:sqlite",
                provide: { targets: { database: "sqlite" } },
            },
        },
        {
            extends: true,
            test: {
                globalSetup: ["setup.ts"],
                name: "simulation:postgresql",
                provide: { targets: { database: "postgresql" } },
            },
        },
    ]);
});

test("run the whole configuration once per combination of its targets without declared projects", () => {
    // declare a matrix and a setup on the root alone
    const configuration = defineConfiguration({
        targets: { database: ["sqlite", "postgresql"] },
        test: { globalSetup: ["setup.ts"], include: ["tests/**"] },
    });

    // expect the root's options as they are, and one extending project per value with the setup
    expect([
        configuration.test?.include,
        configuration.test?.globalSetup,
        configuration.test?.projects,
    ]).toEqual([
        ["tests/**"],
        undefined,
        [
            {
                extends: true,
                test: {
                    globalSetup: ["setup.ts"],
                    name: "sqlite",
                    provide: { targets: { database: "sqlite" } },
                },
            },
            {
                extends: true,
                test: {
                    globalSetup: ["setup.ts"],
                    name: "postgresql",
                    provide: { targets: { database: "postgresql" } },
                },
            },
        ],
    ]);
});
