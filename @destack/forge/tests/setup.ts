import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { type ImageIndex, TestRun } from "@destack/db/test/run";

/** The test runner with provided values that pass the builds to each test. */
declare module "@destack/test" {
    /** The values the global setup provides to every test. */
    export interface ProvidedContext {
        /** The directory with the run's fixture builds, by package name and version. */
        builds: string;
    }
}

/** Write the templates and build each fixture release once per run, removed at its end. */
export default async function setup(project: {
    readonly provide: ((key: "builds", value: string) => void) &
        ((key: "templates", value: ImageIndex) => void);
}): Promise<() => Promise<void>> {
    // write the templates the tests and the builds copy
    const run = await TestRun.write(new URL("./templates.ts", import.meta.url));

    // build the releases in a separate process with the runtime's module resolution
    const builds = await mkdtemp(join(tmpdir(), "destack-forge-builds-"));
    const child = Bun.spawn(
        [
            process.execPath,
            fileURLToPath(new URL("./build.ts", import.meta.url)),
            builds,
            run.index,
        ],
        { stdout: "inherit", stderr: "pipe" },
    );
    const [code, stderr] = await Promise.all([child.exited, new Response(child.stderr).text()]);

    // remove the directory of a failed build, and the templates
    if (code !== 0) {
        await rm(builds, { recursive: true });
        await run.remove();
        throw new Error(`building the fixture releases failed with ${code}: ${stderr}`);
    }

    // provide the templates and the builds to the tests, removing both at the run's end
    project.provide("templates", run.images);
    project.provide("builds", builds);

    return async () => {
        await rm(builds, { recursive: true });
        await run.remove();
    };
}
