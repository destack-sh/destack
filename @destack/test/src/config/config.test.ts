import { testHost } from "../host/host.ts";
import { TEST_START, testSimulation } from "../world/world.ts";
import { expect, test } from "vitest";
import { defineConfiguration } from "./config.ts";

/** The modules every project leaves to Bun: those in the pinned cache directory. */
const SERVER = { deps: { external: [/\/cache\//u] } };

/** The options a unit project takes from its tier. */
const UNIT = {
    testTimeout: 5000,
    hookTimeout: 5000,
    slowTestThreshold: 100,
    expect: { poll: { interval: 5, timeout: 5000 } },
};

/** The options a scenario project takes from its tier. */
const SCENARIO = {
    testTimeout: 60_000,
    hookTimeout: 60_000,
    slowTestThreshold: 5000,
    expect: { poll: { interval: 5, timeout: 60_000 } },
};

/** Run a world whose person's cache directory is pinned, as the configuration reads it. */
function cachedSimulation() {
    const host = testHost();

    return testSimulation(TEST_START, {
        process: {
            arguments: [],
            variables: { XDG_CACHE_HOME: "/cache" },
            workingDirectory: "/",
            id: 1,
            userId: host.process.userId,
            executable: host.process.executable,
            script: host.process.script,
            runtime: host.process.runtime,
        },
    });
}

test("run each project once per combination of its targets, named after them, with the root's setup in each extending project", async () => {
    await using simulation = cachedSimulation();

    // declare a root matrix, a project with targets and a colored label, and one extending the root
    const configuration = defineConfiguration(
        {
            targets: { database: ["sqlite", "postgresql"] },
            test: {
                globalSetup: ["setup.ts"],
                projects: [
                    {
                        extends: false,
                        tier: "unit",
                        targets: { database: ["sqlite"], runtime: ["bun", "node"] },
                        test: { name: { label: "unit", color: "green" }, include: ["src/**"] },
                    },
                    { extends: true, tier: "scenario", test: { name: "simulation" } },
                ],
            },
        },
        simulation.environment("test"),
    );

    // expect one project per combination with its values, and the root's setup in extending ones
    expect(configuration.test?.projects).toEqual([
        {
            extends: false,
            test: {
                name: { label: "unit:sqlite,bun", color: "green" },
                include: ["src/**"],
                ...UNIT,
                provide: { targets: { database: "sqlite", runtime: "bun" } },
                server: SERVER,
            },
        },
        {
            extends: false,
            test: {
                name: { label: "unit:sqlite,node", color: "green" },
                include: ["src/**"],
                ...UNIT,
                provide: { targets: { database: "sqlite", runtime: "node" } },
                server: SERVER,
            },
        },
        {
            extends: true,
            test: {
                globalSetup: ["setup.ts"],
                ...SCENARIO,
                name: "simulation:sqlite",
                provide: { targets: { database: "sqlite" } },
                server: SERVER,
            },
        },
        {
            extends: true,
            test: {
                globalSetup: ["setup.ts"],
                ...SCENARIO,
                name: "simulation:postgresql",
                provide: { targets: { database: "postgresql" } },
                server: SERVER,
            },
        },
    ]);
});

test("run the whole configuration once per combination of its targets without declared projects", async () => {
    await using simulation = cachedSimulation();

    // declare a matrix and a setup on the root alone
    const configuration = defineConfiguration(
        {
            tier: "unit",
            targets: { database: ["sqlite", "postgresql"] },
            test: { globalSetup: ["setup.ts"], include: ["tests/**"] },
        },
        simulation.environment("test"),
    );

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
                    ...UNIT,
                    name: "sqlite",
                    provide: { targets: { database: "sqlite" } },
                    server: SERVER,
                },
            },
            {
                extends: true,
                test: {
                    globalSetup: ["setup.ts"],
                    ...UNIT,
                    name: "postgresql",
                    provide: { targets: { database: "postgresql" } },
                    server: SERVER,
                },
            },
        ],
    ]);
});
