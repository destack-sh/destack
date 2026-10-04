import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import type { TestProject } from "vitest/node";

/** The test runner with provided values that pass the builds to each test. */
declare module "vitest" {
    /** The values the global setup provides to every test. */
    export interface ProvidedContext {
        /** The directory with the run's fixture builds, by package name and version. */
        builds: string;
    }
}

/** Build each fixture release once per run into a directory the run removes at its end. */
export default async function setup(project: TestProject): Promise<() => Promise<void>> {
    // build in a separate process, since the compiler resolves modules as the runtime does
    const builds = await mkdtemp(join(tmpdir(), "destack-forge-builds-"));
    const child = Bun.spawn(
        [process.execPath, fileURLToPath(new URL("./build.ts", import.meta.url)), builds],
        { stdout: "inherit", stderr: "pipe" },
    );
    const [code, stderr] = await Promise.all([child.exited, new Response(child.stderr).text()]);
    if (code !== 0) {
        throw new Error(`building the fixture releases failed with ${code}: ${stderr}`);
    }
    project.provide("builds", builds);

    return () => rm(builds, { recursive: true });
}
