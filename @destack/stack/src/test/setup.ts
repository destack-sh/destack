import { CompilerPool } from "@destack/build";
import { PackageCopy, testStore } from "@destack/build/test";
import { TestRun } from "@destack/db/test/run";

/** The fixtures whose closures the compile tests build on. */
const FIXTURES = ["resource", "stack", "workload"].map(
    (name) => new URL(`../../tests/fixture/${name}/source/`, import.meta.url),
);

/** Write stack's test templates once per run. */
const templates = TestRun.setup(new URL("./templates.ts", import.meta.url));

/** Write the test templates and store the fixtures' dependency builds once per run. */
export default async function setup(
    project: Parameters<typeof templates>[0],
): Promise<() => Promise<void>> {
    // write the templates, then store each fixture's dependency builds
    const remove = await templates(project);
    await using store = await testStore();
    await using compilers = await CompilerPool.open(store);
    for (const fixture of FIXTURES) {
        const source = await PackageCopy.open(fixture, compilers);
        await source[Symbol.asyncDispose]();
    }

    return remove;
}
