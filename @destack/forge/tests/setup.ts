import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

/** The test runner with provided values that pass the builds to each test. */
declare module "@destack/test" {
    /** The values the global setup provides to every test. */
    export interface ProvidedContext {
        /** The directory with the run's fixture builds, by package name and version. */
        builds: string;
    }
}

/** Build each fixture release once per run into a directory the run removes at its end. */
export default async function setup(project: {
    readonly provide: (key: "builds", value: string) => void;
}): Promise<() => Promise<void>> {
    // build the releases in a separate process with the runtime's module resolution
    const builds = await mkdtemp(join(tmpdir(), "destack-forge-builds-"));
    const child = Bun.spawn(
        [process.execPath, fileURLToPath(new URL("./build.ts", import.meta.url)), builds],
        { stdout: "inherit", stderr: "pipe" },
    );
    const [code, stderr] = await Promise.all([child.exited, new Response(child.stderr).text()]);

    // remove the directory of a failed build
    if (code !== 0) {
        await rm(builds, { recursive: true });
        throw new Error(`building the fixture releases failed with ${code}: ${stderr}`);
    }

    // provide the builds to the tests, and remove them at the run's end
    project.provide("builds", builds);

    return () => rm(builds, { recursive: true });
}
