import { testSimulation } from "../test/world.ts";
import type { FileSystem } from "@destack/environment";
import { join } from "node:path";
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
    const read = () => ModuleClosure.read(join(root, "app", "entry.ts"), environment);
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
        initial: initial.releases,
        edited: edited.releases,
        released: released.releases,
        selected: released.select(new Set(["other"])).releases,
    }).toEqual({
        initial: ["dep@1.0.0"],
        edited: ["dep@1.0.0"],
        released: ["dep@1.0.1"],
        selected: [],
    });
});
