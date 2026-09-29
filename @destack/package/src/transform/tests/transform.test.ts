import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, onTestFinished, test } from "@destack/test";
import { PackageLocator, type ModulePackage } from "../locator.ts";
import { transformModule } from "../transform.ts";
import { ModuleMetadata } from "../../definition/metadata.ts";

/** The package whose metadata the transform injects. */
const owner: ModulePackage = {
    directory: "/package",
    metadata: ModuleMetadata.parse({
        package: {
            id: "package-01996ab0-0000-7000-8000-000000000001",
            name: "@example/notes",
            version: "2026.9.0",
        },
    }),
};

/** A module inside the fixture package declaring defineNote with its module third. */
const NOTE_PATH = fileURLToPath(new URL("fixture/notes/note.ts", import.meta.url));

test("stamp declaration constructors and metadata reads with the calling module", () => {
    const code = [
        'import { defineNote } from "./declare.ts";',
        'import { definePackage } from "@destack/package";',
        'import * as notes from "./declare.ts";',
        'export const note = defineNote("note");',
        'export const other = notes.defineNote("other");',
        'export const owned = defineNote("owned", {}, import.meta.destack);',
        "export const notes = definePackage({});",
        "export const id = import.meta.destack.package.id;",
    ].join("\n");

    // pad omitted options before the module and keep explicit modules
    const metadata = JSON.stringify(owner.metadata);
    const result = transformModule(code, NOTE_PATH, owner, new PackageLocator())!.code;
    expect(result.split("\n")).toEqual([
        `const __destackModule = Object.freeze(${metadata});`,
        'import { defineNote } from "./declare.ts";',
        'import { definePackage } from "@destack/package";',
        'import * as notes from "./declare.ts";',
        'export const note = defineNote("note", undefined, __destackModule);',
        'export const other = notes.defineNote("other", undefined, __destackModule);',
        'export const owned = defineNote("owned", {}, __destackModule);',
        "export const notes = definePackage({}, __destackModule);",
        "export const id = __destackModule.package.id;",
    ]);
    expect(
        transformModule("export const plain = 1;", NOTE_PATH, owner, new PackageLocator()),
    ).toBeUndefined();
});

test("read a package's constructors once per locator, and its changed ones in the next", async () => {
    // declare one constructor in a package of a temporary directory
    const directory = await mkdtemp(join(tmpdir(), "destack-constructors-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    const declare = (declarations: Readonly<Record<string, { module: number }>>) =>
        writeFile(
            join(directory, "destack.json"),
            JSON.stringify({ id: owner.metadata.package.id, declarations }),
        );
    await writeFile(join(directory, "package.json"), JSON.stringify({ name: "@example/fresh" }));
    await declare({ defineNote: { module: 1 } });
    const build = new PackageLocator();
    const before = build.constructors("@example/fresh", directory);

    // keep the first build's answer after the change, and read the change in the next build
    await declare({ defineNote: { module: 1 }, defineTag: { module: 2 } });
    const cached = build.constructors("@example/fresh", directory);
    const next = new PackageLocator().constructors("@example/fresh", directory);
    expect([before, cached, next].map((constructors) => Object.keys(constructors))).toEqual([
        ["defineNote"],
        ["defineNote"],
        ["defineNote", "defineTag"],
    ]);
});
