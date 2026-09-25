import { fileURLToPath } from "node:url";
import { expect, test } from "@destack/test";
import { transformModule } from "../transform.ts";
import { ModuleMetadata } from "../../definition/metadata.ts";

/** The metadata the transform injects. */
const metadata = ModuleMetadata.parse({
    package: {
        id: "package-01996ab0-0000-7000-8000-000000000001",
        name: "@example/notes",
        version: "2026.9.0",
    },
});

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

    // pad omitted options before the module in named and namespace calls, keep explicit modules
    const result = transformModule(code, NOTE_PATH, metadata)!.code;
    expect(result.split("\n").slice(1)).toEqual([
        'import { defineNote } from "./declare.ts";',
        'import { definePackage } from "@destack/package";',
        'import * as notes from "./declare.ts";',
        'export const note = defineNote("note", undefined, __destackModule);',
        'export const other = notes.defineNote("other", undefined, __destackModule);',
        'export const owned = defineNote("owned", {}, __destackModule);',
        "export const notes = definePackage({}, __destackModule);",
        "export const id = __destackModule.package.id;",
    ]);
    expect(transformModule("export const plain = 1;", NOTE_PATH, metadata)).toBeUndefined();
});
