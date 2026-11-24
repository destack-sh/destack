import { Simulation } from "@destack/environment/simulation";
import { Distribution } from "@destack/update";
import { expect, test } from "@destack/test";
import { CompilerFingerprint } from "./fingerprint.ts";
import { testSimulation } from "../test/world.ts";
import type { FileSystem } from "@destack/environment";
import { join } from "node:path";
import { testWorldOptions } from "@destack/test/world";

/** The resources of a release's application, beside its helpers. */
const RESOURCES = "/Applications/Destack.app/Contents/Resources";

/** The bundle a release's compiler runs, embedded in its executable. */
const SCRIPT = "/$bunfs/root/destack";

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
            executable: "/Applications/Destack.app/Contents/Helpers/destack",
            script: SCRIPT,
            runtime: { revision, isStandalone: true },
        },
    });
    const environment = simulation.environment("compiler");
    await environment.files.mkdir("/$bunfs/root", { recursive: true });
    await environment.files.mkdir(RESOURCES, { recursive: true });
    await environment.files.writeTextFile(SCRIPT, bundle);
    const shipped = Distribution.toolchainManifest(environment.host.architecture);
    await environment.files.writeTextFile(`${RESOURCES}/${shipped}`, manifest);

    return await CompilerFingerprint.of(environment).digest();
}

test("identify a release compiler by its version, runtime build and toolchain, a changed bundle alike", async () => {
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

    // key a changed bundle of the same version alike, as its version says what it outputs, and the rest apart
    const [first, again, bundled, runtimed, tooled] = identified;
    expect({
        isAlike: first === again && first === bundled,
        isEachApart: runtimed !== first && tooled !== first && runtimed !== tooled,
    }).toEqual({ isAlike: true, isEachApart: true });
});

/** Write files below a directory by their relative paths. */
async function writeFiles(
    root: string,
    contents: Readonly<Record<string, string>>,
    files: FileSystem,
): Promise<void> {
    for (const [path, text] of Object.entries(contents)) {
        await files.mkdir(join(root, path, ".."), { recursive: true });
        await files.writeTextFile(join(root, path), text);
    }
}

test("list the npm releases an entry loads through its sources, a release's version changing them", async () => {
    // write an application importing a module and a linked workspace library, which imports an npm release
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const root = await files.realPath(await files.makeTemporaryDirectory("destack-closure-"));
    await writeFiles(
        root,
        {
            "app/package.json": JSON.stringify({ name: "app", version: "1.0.0" }),
            "app/entry.ts":
                'import "./used.ts";\nimport { lib } from "lib";\nimport type { Unused } from "./typed.ts";\n',
            "app/used.ts": "export {};\n",
            "app/typed.ts": 'import "typed-only";\nexport type Unused = string;\n',
            "lib/package.json": JSON.stringify({
                name: "lib",
                version: "1.0.0",
                exports: "./index.ts",
            }),
            "lib/index.ts": 'import "dep";\nexport const lib = 1;\n',
            "node_modules/dep/package.json": JSON.stringify({
                name: "dep",
                version: "1.0.0",
                main: "index.js",
            }),
            "node_modules/dep/index.js": "module.exports = 1;\n",
        },
        files,
    );
    await files.mkdir(join(root, "app", "node_modules"));
    await files.symlink(join(root, "lib"), join(root, "app", "node_modules", "lib"));
    const read = () => CompilerFingerprint.releases(join(root, "app", "entry.ts"), environment);
    const initial = await read();

    // edit a loaded source, then the npm release's version
    await writeFiles(root, { "lib/index.ts": 'import "dep";\nexport const lib = 2;\n' }, files);
    const edited = await read();
    await writeFiles(
        root,
        {
            "node_modules/dep/package.json": JSON.stringify({
                name: "dep",
                version: "1.0.1",
                main: "index.js",
            }),
        },
        files,
    );
    const released = await read();

    // list the release the sources reach, the same after a source edit, another after its version moves
    expect({
        initial,
        edited,
        released,
    }).toEqual({
        initial: ["dep@1.0.0"],
        edited: ["dep@1.0.0"],
        released: ["dep@1.0.1"],
    });
});
