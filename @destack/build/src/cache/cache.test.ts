import { Simulation } from "@destack/environment/simulation";
import { expect, test } from "@destack/test";
import { Toolchain } from "./cache.ts";

/** The bundle a release's compiler runs, as its launcher starts it. */
const SCRIPT = "/Applications/Destack.app/Contents/Resources/program/destack-build.js";

/** Identify a release compiler running a bundle on a runtime build. */
async function identify(revision: string, bundle: string): Promise<string> {
    // start a release's compiler process on the runtime build, its bundle written in place
    await using simulation = new Simulation({
        seed: 1,
        start: 0,
        process: {
            arguments: [],
            variables: {},
            workingDirectory: "/",
            id: 1,
            userId: 501,
            executable: "/Applications/Destack.app/Contents/Helpers/bun",
            script: SCRIPT,
            runtime: { revision, isStandalone: true },
        },
    });
    const environment = simulation.environment("compiler");
    await environment.files.mkdir("/Applications/Destack.app/Contents/Resources/program", {
        recursive: true,
    });
    await environment.files.writeTextFile(SCRIPT, bundle);

    return await Toolchain.of(environment).digest();
}

test("identify a release compiler by its runtime build and its bundle, the same pair alike", async () => {
    // identify one compiler twice, another bundle, and the same bundle on another runtime build
    const runtime = "a".repeat(40);
    const identified = [
        await identify(runtime, "compile(1)"),
        await identify(runtime, "compile(1)"),
        await identify(runtime, "compile(2)"),
        await identify("b".repeat(40), "compile(1)"),
    ];

    // key the same pair alike and every change apart
    const [first, again, bundle, rebuilt] = identified;
    expect([
        first === again,
        new Set(identified).size,
        bundle !== first,
        rebuilt !== first,
    ]).toEqual([true, 3, true, true]);
});
