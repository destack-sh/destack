import { fileURLToPath } from "node:url";
import { expect, test } from "@destack/test";
import { transformModule } from "../transform.ts";
import { ModuleMetadata } from "../../definition/metadata.ts";
import type { ModulePackage } from "../transform.ts";

/** The package whose metadata and views the transform injects. */
const owner: ModulePackage = {
    directory: "/package",
    metadata: ModuleMetadata.parse({
        package: {
            id: "package-01996ab0-0000-7000-8000-000000000001",
            name: "@example/notes",
            version: "2026.9.0",
        },
    }),
    views: { home: { entrypoint: "./view" } },
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

    // pad omitted options before the module, keep explicit modules, and give definePackage the views
    const metadata = JSON.stringify(owner.metadata);
    const stamped = JSON.stringify({ ...owner.metadata, views: owner.views });
    const result = transformModule(code, NOTE_PATH, owner)!.code;
    expect(result.split("\n")).toEqual([
        `const __destackPackage = Object.freeze(${stamped});`,
        `const __destackModule = Object.freeze(${metadata});`,
        'import { defineNote } from "./declare.ts";',
        'import { definePackage } from "@destack/package";',
        'import * as notes from "./declare.ts";',
        'export const note = defineNote("note", undefined, __destackModule);',
        'export const other = notes.defineNote("other", undefined, __destackModule);',
        'export const owned = defineNote("owned", {}, __destackModule);',
        "export const notes = definePackage({}, __destackPackage);",
        "export const id = __destackModule.package.id;",
    ]);
    expect(transformModule("export const plain = 1;", NOTE_PATH, owner)).toBeUndefined();
});
