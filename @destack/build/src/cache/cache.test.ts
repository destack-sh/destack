import { Simulation } from "@destack/environment/simulation";
import { Distribution } from "@destack/update";
import { expect, test } from "@destack/test";
import { CompilerFingerprint } from "./cache.ts";
import { testWorldOptions } from "@destack/test/world";

/** The resources of a release's application, beside its helpers. */
const RESOURCES = "/Applications/Destack.app/Contents/Resources";

/** The bundle a release's compiler runs, as its launcher starts it. */
const SCRIPT = `${RESOURCES}/program/destack-build.js`;

/** Identify a release compiler by its runtime build, its bundle and its toolchain manifest. */
async function identify(revision: string, bundle: string, manifest: string): Promise<string> {
    // start a release's compiler process on the runtime build, its bundle written in place in memory
    await using simulation = new Simulation({
        world: testWorldOptions(),
        seed: 1,
        start: 0,
        files: "memory",
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
    await environment.files.mkdir(`${RESOURCES}/program`, {
        recursive: true,
    });
    await environment.files.writeTextFile(SCRIPT, bundle);
    const shipped = Distribution.toolchainManifest(environment.host.architecture);
    await environment.files.writeTextFile(`${RESOURCES}/${shipped}`, manifest);

    return await CompilerFingerprint.of(environment).digest();
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
