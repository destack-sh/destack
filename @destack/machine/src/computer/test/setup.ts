import { BuildStore } from "@destack/build/store";
import { CompilerPool } from "@destack/build";
import { testStore } from "@destack/build/test";
import { TestRun } from "@destack/db/test/run";
import { canonicalize } from "@destack/schema";
import type { TestProject } from "@destack/test";
import { testHost } from "@destack/test/host";
import {
    BUILDS_VARIABLE,
    PLATFORM_PACKAGES,
    STACK_FIXTURE,
    START_FIXTURE,
    TestDistribution,
} from "./distribution.ts";
import { SCENARIO_FIXTURES, WORLD_FIXTURES, WorldDriver } from "./world.ts";

/** The module of machine's test templates, which the run writes once. */
const TEMPLATES = new URL("./templates.ts", import.meta.url);

/** Build the platform, the stacks and the fixtures, write the templates and make a trash. */
export default async function setup(project: TestProject): Promise<() => Promise<void>> {
    // build the platform's packages, the stacks and the fixtures at once, so each world only reads the run's store
    const built: Record<string, string> = {};
    const scenario: Record<string, string> = {};
    {
        await using store = await testStore();
        await using compilers = await CompilerPool.open(store);
        const { crypto } = store.environment;
        const builds = [
            ...PLATFORM_PACKAGES.map(
                (name) => [name, built, () => TestDistribution.platform(name, compilers)] as const,
            ),
            ...[...WORLD_FIXTURES, STACK_FIXTURE, START_FIXTURE, ...SCENARIO_FIXTURES].map(
                (name) =>
                    [
                        name,
                        SCENARIO_FIXTURES.some((each) => each === name) ? scenario : built,
                        () => WorldDriver.build(name, compilers),
                    ] as const,
            ),
        ];
        await Promise.all(
            builds.map(async ([name, digests, build]) => {
                digests[name] = await BuildStore.digest((await build()).manifest, crypto);
            }),
        );
    }

    // hand the digests to the tests, and to the templates, whose distributions ship the platform's builds
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
