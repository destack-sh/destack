import { testSimulation } from "../test/world.ts";
import type { FileSystem } from "@destack/environment";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "@destack/test";
import { ModuleClosure } from "./closure.ts";

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

test("key an entry's closure by the sources it loads and the npm releases it requires", async () => {
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
            "app/typed.ts": "export type Unused = string;\n",
            "app/unused.ts": "export {};\n",
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
    const digest = async () =>
        await (
            await ModuleClosure.read(join(root, "app", "entry.ts"), environment)
        ).digest(environment.crypto);
    const initial = await digest();

    // edit what the entry does not load, then the npm release's bytes, then a loaded source and the release's version
    await writeFiles(
        root,
        {
            "app/unused.ts": "export const edited = 1;\n",
            "app/typed.ts": "export type Unused = number;\n",
            "node_modules/dep/index.js": "module.exports = 2;\n",
        },
        files,
    );
    const unloaded = await digest();
    await writeFiles(root, { "lib/index.ts": 'import "dep";\nexport const lib = 2;\n' }, files);
    const source = await digest();
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
    const release = await digest();

    // ignore unloaded modules and a release's bytes, change the digest for a loaded source and a version
    expect({
        isUnloadedUnchanged: unloaded === initial,
        isSourceChanged: source !== unloaded,
        isReleaseChanged: release !== source,
        modules: [
            ...(
                await ModuleClosure.read(join(root, "app", "entry.ts"), environment)
            ).modules.keys(),
        ],
    }).toEqual({
        isUnloadedUnchanged: true,
        isSourceChanged: true,
        isReleaseChanged: true,
        modules: ["app/entry.ts", "app/used.ts", "dep@1.0.1", "lib/index.ts"],
    });
});

test("key the toolchain by the compiler's modules, leaving the modules only its host loads out", async () => {
    // read the closure of the entry a compiler process starts from, in a world
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const closure = await ModuleClosure.read(
        fileURLToPath(new URL("../main.ts", import.meta.url)),
        environment,
    );
    const modules = new Set(closure.modules.keys());

    // include the compiler and its conversion pass, and leave the local store and its bucket out
    expect({
        worker: modules.has("@destack/build/src/build/worker.ts"),
        conversion: modules.has("@destack/build/src/compile/conversion.ts"),
        store: modules.has("@destack/build/src/store/local.ts"),
        bucket: [...modules].some((module) => module.startsWith("@destack/bucket/")),
    }).toEqual({ worker: true, conversion: true, store: false, bucket: false });
});
