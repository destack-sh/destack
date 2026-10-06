import { fileURLToPath } from "node:url";
import { expect, test } from "@destack/test";
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
    const transformed = transformModule(code, NOTE_PATH, owner, new PackageLocator());
    if (transformed === undefined) {
        throw new TypeError("the module was left unchanged");
    }
    const result = transformed.code;
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

test("pass the calling module to stamped tags and to members of named and namespace imports", () => {
    const code = [
        'import { Note, note } from "./declare.ts";',
        'import * as notes from "./declare.ts";',
        "export const greeting = note`Hello ${name}`;",
        'export const title = Note.titled("heading")`Trips`;',
        'export const other = notes.Note.titled("heading")`Work`;',
        "export const plain = notes.note`Plain`;",
        'export const owned = Note.titled("heading", import.meta.destack)`Owned`;',
    ].join("\n");

    // call bare tags with the module, and append it to calls building tags
    const metadata = JSON.stringify(owner.metadata);
    const transformed = transformModule(code, NOTE_PATH, owner, new PackageLocator());
    expect(transformed?.code.split("\n")).toEqual([
        `const __destackModule = Object.freeze(${metadata});`,
        'import { Note, note } from "./declare.ts";',
        'import * as notes from "./declare.ts";',
        "export const greeting = note(__destackModule)`Hello ${name}`;",
        'export const title = Note.titled("heading", __destackModule)`Trips`;',
        'export const other = notes.Note.titled("heading", __destackModule)`Work`;',
        "export const plain = notes.note(__destackModule)`Plain`;",
        'export const owned = Note.titled("heading", __destackModule)`Owned`;',
    ]);
});
