import { join } from "node:path";
import type { TestProject } from "vitest/node";
import { CompilerPool, type PackageBuild } from "@destack/build";
import { testStore } from "@destack/build/test";
import { TestDatabase } from "@destack/db/test";
import { TestRun } from "@destack/db/test/run";
import { nth } from "@destack/schema";
import {
    ANSWER_VERSIONS,
    FIXTURE_VERSION,
    type FixturePackage,
    FixtureSource,
    PRERELEASE_VERSION,
} from "./fixture/package.ts";
import { testHost } from "@destack/test/host";

/** The test runner with provided values that pass the builds to each test. */
declare module "vitest" {
    /** The values the global setup provides to every test. */
    export interface ProvidedContext {
        /** The directory with the run's fixture builds, by package name and version. */
        builds: string;
    }
}

/** Write the templates and build each fixture release once per run, removed at its end. */
export default async function setup(project: TestProject): Promise<() => Promise<void>> {
    // write the templates the tests copy, in the project's database
    const dialect = TestDatabase.target(project);
    const run = await TestRun.write(new URL("./templates.ts", import.meta.url), dialect);

    // build the releases into a directory, through the store builds keep across runs
    const { files } = testHost();
    const builds = await files.makeTemporaryDirectory("destack-forge-builds-");
    await build(builds);

    // provide the templates and the builds to the tests, releasing the templates and removing the builds at the run's end
    project.provide("templates", run.images);
    project.provide("builds", builds);

    return async () => {
        await files.remove(builds, { recursive: true });
        await run.release();
    };
}

/** Build every fixture release into a directory by package name and version. */
async function build(builds: string): Promise<void> {
    // copy the sources, building on one pool of compilers over the persistent test store
    const host = testHost();
    const sources = await host.files.makeTemporaryDirectory("destack-forge-sources-");
    try {
        await using store = await testStore();
        await using compilers = await CompilerPool.open(store);

        // build answer's releases, then greeting's on answer's first
        const answer = await FixtureSource.open("answer", sources, compilers);
        const answers = await answer.build([], ANSWER_VERSIONS);
        const greeting = await FixtureSource.open("greeting", sources, compilers);
        const greetings = await greeting.build([nth(answers, 0)], [FIXTURE_VERSION]);

        // keep answer's last version and a dev prerelease, both compiled without a commit
        const uncommitted = await answer.uncommitted();
        const prerelease = await answer.uncommitted(PRERELEASE_VERSION);

        // keep each build by package and version, the uncommitted one by its state
        const kept: readonly (readonly [FixturePackage, string, PackageBuild])[] = [
            ...answers.map((built) => ["answer", built.manifest.package.version, built] as const),
            ...greetings.map(
                (built) => ["greeting", built.manifest.package.version, built] as const,
            ),
            ["answer", "uncommitted", uncommitted],
            ["answer", PRERELEASE_VERSION, prerelease],
        ];
        for (const [name, version, built] of kept) {
            await host.files.copy(built.directory, join(builds, name, version));
        }
    } finally {
        await host.files.remove(sources, { recursive: true });
    }
}
