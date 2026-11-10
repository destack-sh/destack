import { Simulation } from "@destack/environment/simulation";
import { Toolchain } from "@destack/check/toolchain";
import { expect, test } from "@destack/test";
import { CompilerIdentity } from "./cache.ts";

/** The bundle a release's compiler runs, as its launcher starts it. */
const SCRIPT = "/Applications/Destack.app/Contents/Resources/program/destack-build.js";

/** The directory of the checked-out toolchains, each named by its manifest's digest. */
const TOOLCHAINS = "/Users/ada/.destack/toolchain";

/** Identify a release compiler running a bundle on a runtime build with a toolchain. */
async function identify(revision: string, bundle: string, manifest: string): Promise<string> {
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
    const toolchain = async () => Toolchain.at(`${TOOLCHAINS}/${manifest}`);

    return await CompilerIdentity.of(environment, toolchain).digest();
}

test("identify a release compiler by its runtime build, its bundle and its toolchain, the same alike", async () => {
    // identify one compiler twice, then change its bundle, its runtime build and its toolchain one at a time
    const runtime = "a".repeat(40);
    const manifest = "c".repeat(64);
    const identified = [
        await identify(runtime, "compile(1)", manifest),
        await identify(runtime, "compile(1)", manifest),
        await identify(runtime, "compile(2)", manifest),
        await identify("b".repeat(40), "compile(1)", manifest),
        await identify(runtime, "compile(1)", "d".repeat(64)),
    ];

    // key the same compiler alike and every change apart
    const [first, again, ...changed] = identified;
    expect({
        isAlike: first === again,
        distinct: new Set(identified).size,
        isEachApart: changed.every((each) => each !== first),
    }).toEqual({ isAlike: true, distinct: 4, isEachApart: true });
});
