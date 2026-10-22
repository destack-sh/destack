import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, onTestFinished, test } from "@destack/test";
import { PackageSource, readPackageJson } from "./source.ts";

/** Write a package whose destack.json overrides the runtimes of some exports. */
async function writePackage(overrides: readonly string[]): Promise<string> {
    const directory = await mkdtemp(join(tmpdir(), "destack-source-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    await writeFile(
        join(directory, "package.json"),
        JSON.stringify({
            name: "@example/source",
            version: "2026.9.0",
            exports: { ".": "./src/index.ts", "./server": "./src/server.ts" },
        }),
    );
    await writeFile(
        join(directory, "destack.json"),
        JSON.stringify({
            $schema: "https://destack.app/schemas/2026.9.0/destack.json",
            id: "package-01996ab0-0000-7000-8000-000000000a01",
            language: "typescript",
            runtimes: ["bun"],
            exports: Object.fromEntries(overrides.map((name) => [name, { runtimes: ["workerd"] }])),
        }),
    );

    return directory;
}

test("read runtime overrides of a package's exports, and refuse one of an export package.json lacks", async () => {
    // read an override of an export, and refuse one of a missing export
    const read = await readPackageJson(await writePackage(["./server"]));
    await expect(readPackageJson(await writePackage(["./gone"]))).rejects.toMatchObject({
        code: "INVALID_DEFINITION",
        message: "destack.json overrides the runtimes of an export package.json lacks: ./gone",
    });
    expect(read.definition.exports).toEqual({ "./server": { runtimes: ["workerd"] } });
});

test("expand a pattern export into the modules its target matches, nested ones included", async () => {
    // write a package exporting its icons by a pattern, beside a file the pattern leaves out
    const directory = await writePackage([]);
    await writeFile(
        join(directory, "package.json"),
        JSON.stringify({
            name: "@example/source",
            version: "2026.9.0",
            exports: { ".": "./src/index.ts", "./icons/*": "./src/icons/*.ts" },
        }),
    );
    await mkdir(join(directory, "src", "icons", "nested"), { recursive: true });
    for (const path of ["index.ts", "icons/a.ts", "icons/nested/b.ts", "icons/readme.md"]) {
        await writeFile(join(directory, "src", path), "export {};\n");
    }

    // open it, naming each matched module by the text the wildcard stands for
    await using source = await PackageSource.open({
        directory,
        runtime: "bun",
        tree: {},
        roots: "package",
    });
    expect(Object.keys(source.exports).toSorted()).toEqual([".", "./icons/a", "./icons/nested/b"]);
});
