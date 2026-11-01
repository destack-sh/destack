import { BuildStore } from "@destack/build/store";
import { testStore } from "@destack/build/test";
import { TestRun } from "@destack/db/test/run";
import type { TestProject } from "@destack/test";
import { testHost } from "@destack/test/host";
import { FIXTURES_VARIABLE } from "./distribution.ts";
import { WORLD_FIXTURES, WorldDriver } from "./world.ts";

/** The module of machine's test templates, which the run writes once. */
const TEMPLATES = new URL("./templates.ts", import.meta.url);

/** Build the fixtures, write the templates and make a trash, per run. */
export default async function setup(project: TestProject): Promise<() => Promise<void>> {
    // build the fixtures worlds install, so each world only reads the run's store
    const fixtures: Record<string, string> = {};
    {
        await using store = await testStore();
        for (const fixture of WORLD_FIXTURES) {
            const built = await WorldDriver.build(fixture, store);
            fixtures[fixture] = await BuildStore.digest(built.manifest, store.environment.crypto);
        }
    }

    // hand their digests to the tests and to the templates, whose distributions ship what the fixtures were built against
    project.provide("fixtures", fixtures);
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
