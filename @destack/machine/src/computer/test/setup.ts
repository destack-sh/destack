import { CompilerPool, Workspace } from "@destack/build";
import { fileURLToPath } from "node:url";
import { testStore } from "@destack/build/test";
import { TestRun } from "@destack/db/test/run";
import { canonicalize, type Digest } from "@destack/schema";
import type { TestProject } from "@destack/test";
import { testHost } from "@destack/test/host";
import {
    BUILDS_VARIABLE,
    DESKTOP_PACKAGE,
    PLATFORM_PACKAGES,
    RUNTIME_PACKAGE,
    STACK_FIXTURE,
    START_FIXTURE,
    TestDistribution,
} from "./distribution.ts";
import { SCENARIO_FIXTURES, WORLD_FIXTURES } from "./fixture.ts";
import { WorldDriver } from "./world.ts";
import { Toolchain } from "@destack/check/toolchain";
import { COMPUTER_RUNTIMES } from "../computer.ts";

/** The module of machine's test templates, which the run writes once. */
const TEMPLATES = new URL("./templates.ts", import.meta.url);

/** Build the platform, the stacks and the fixtures, write the templates and make a trash. */
export default async function setup(project: TestProject): Promise<() => Promise<void>> {
    // build the platform's packages, the stacks and the fixtures at once, so each world only reads the run's store
    const built: Record<string, Digest> = {};
    const scenario: Record<string, string> = {};
    {
        await using store = await testStore();
        await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
        const workspace = await Workspace.open(
            fileURLToPath(new URL("../../../", import.meta.url)),
            compilers,
        );
        const builds = [
            [RUNTIME_PACKAGE, built, () => TestDistribution.runtime(workspace)] as const,
            [DESKTOP_PACKAGE, built, () => TestDistribution.desktop(workspace)] as const,
            ...PLATFORM_PACKAGES.map(
                (name) => [name, built, () => TestDistribution.platform(name, workspace)] as const,
            ),
            ...[...WORLD_FIXTURES, STACK_FIXTURE, START_FIXTURE, ...SCENARIO_FIXTURES].map(
                (name) =>
                    [
                        name,
                        SCENARIO_FIXTURES.some((each) => each === name) ? scenario : built,
                        () => WorldDriver.build(name, workspace),
                    ] as const,
            ),
        ];
        await Promise.all(
            builds.map(async ([name, digests, build]) => {
                digests[name] = (await build()).lock;
            }),
        );

        // lock the runtime's builtins too
        const { units } = await TestDistribution.runtime(workspace);
        for (const [name, lock] of units) {
            built[name] ??= lock;
        }

        // write the layer every computer ships once here, so no test pays for it
        const shipped = await TestDistribution.shipped(store, built, []);
        await store.writeLinkedLayer(shipped, COMPUTER_RUNTIMES);
    }

    // hand the locks to the tests
    project.provide("built", built);
    project.provide("scenarios", scenario);
    const variables = { [BUILDS_VARIABLE]: canonicalize(built) };
    const remove = await TestRun.setup(TEMPLATES, variables)(project);

    // move the computers' state directories into one trash, removed with the run, as Bazel's execroots
    const { files } = testHost();
    const trash = await files.makeTemporaryDirectory("destack-machine-trash-");
    project.provide("trash", trash);

    return async () => {
        await remove();
        await files.remove(trash, { recursive: true, force: true });
    };
}
