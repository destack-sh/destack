import { BuildStore } from "@destack/build/store";
import { testStore } from "@destack/build/test";
import { TestRun } from "@destack/db/test/run";
import type { TestProject } from "@destack/test";
import { testHost } from "@destack/test/host";
import { FIXTURES_VARIABLE } from "./distribution.ts";
import { SCENARIO_FIXTURES, WORLD_FIXTURES, WorldDriver } from "./world.ts";

/** The module of machine's test templates, which the run writes once. */
const TEMPLATES = new URL("./templates.ts", import.meta.url);

/** Build the fixtures, write the templates and make a trash, per run. */
export default async function setup(project: TestProject): Promise<() => Promise<void>> {
    // build the fixtures and the packages scenarios install, so each world only reads the run's store
    const fixtures: Record<string, string> = {};
    const scenario: Record<string, string> = {};
    {
        await using store = await testStore();
        const packages = [
            ...WORLD_FIXTURES.map((fixture) => [fixture, fixtures] as const),
            ...SCENARIO_FIXTURES.map((each) => [each, scenario] as const),
        ];
        for (const [name, built] of packages) {
            const build = await WorldDriver.build(name, store);
            built[name] = await BuildStore.digest(build.manifest, store.environment.crypto);
        }
    }

    // hand the digests to the tests, and the fixtures' to the templates, whose distributions ship what those were built against
    project.provide("fixtures", fixtures);
    project.provide("scenarios", scenario);
    const variables = { [FIXTURES_VARIABLE]: JSON.stringify(fixtures) };
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
