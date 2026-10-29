import { testHost } from "@destack/test/host";
import { space } from "@destack/account/object";
import { defineObject, field, OBJECT_DECLARATION, ObjectDescription } from "@destack/object";
import objectPackage from "@destack/object/package";
import { graph, Package } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { Attribution, StackFrame, Symbolicator } from "@destack/package/stack";
import { MemoryBuild } from "@destack/package/test";
import { Digest } from "@destack/schema";

/** The notes application, whose builds the test space installs and whose failures it groups. */
export const NOTES = Package.parse({
    id: "package-01996ab0-0000-7000-8000-000000000004",
    name: "@example/notes",
    version: "2026.9.0",
});

/** The notes the notes application keeps, as its note module declares them. */
export const note = defineObject(
    { name: "note", plural: "notes", scope: space, fields: { title: field.string() } },
    undefined,
    { package: NOTES },
);

/** The note module's path in the notes build. */
const NOTE_MODULE = "src/note.ts";

/** The note module's source: a class whose rename method fails. */
const NOTE_SOURCE = [
    "export class Note {",
    "    rename(title: string) {",
    "        return check(title);",
    "    }",
    "}",
].join("\n");

/** The stack trace of a failure of `Note.rename`, as the notes workload's runtime writes it. */
export const RENAME_STACK =
    "RangeError: title is too long\n    at rename (file:///srv/output/bun/workload.js:1:1)";

/** Encode text as UTF-8. */
function encode(value: string): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(value);
}

/** The notes application's fixture: its build and the attributed frames of its failures. */
export const TestNotes = {
    /** Write the notes build: a workload whose source map points into `Note.rename`. */
    async build(): Promise<BuildReader> {
        // keep the source and the workload's source map, generated 1:0 mapping to note.ts 3:8
        const files = new Map<string, Uint8Array<ArrayBuffer>>([
            [NOTE_MODULE, encode(NOTE_SOURCE)],
            [
                "output/bun/workload.js.map",
                encode(
                    JSON.stringify({
                        version: 3,
                        sources: ["../../src/note.ts"],
                        names: ["rename"],
                        mappings: "AAEQA",
                    }),
                ),
            ],
        ]);

        return MemoryBuild.write(
            files,
            {
                package: NOTES,
                sourceMaps: [
                    { generated: "output/bun/workload.js", map: "output/bun/workload.js.map" },
                ],
                declarations: (await notesModule()).declarations,
            },
            testHost().crypto,
        );
    },

    /** Attribute the frames of a `Note.rename` failure, as the machine forwards them. */
    async frames(build: BuildReader): Promise<string> {
        // symbolize the stack, then attribute it by the note module's code and graph
        const symbolized = await new Symbolicator(build).frames(StackFrame.read(RENAME_STACK));
        const index = { code: [await notesCode()], modules: [await notesModule()] };
        const attributed = await new Attribution(build, index).frames(symbolized);

        return JSON.stringify(attributed);
    },
};

/** Index the note module: its `note` object at `Note` and the `rename` method it derives. */
async function notesModule(): Promise<graph.Module> {
    // declare the `note` object at `Note` and the `rename` method it derives
    const object = graph.Declaration.at(symbol("Note"), {
        kind: OBJECT_DECLARATION,
        kindPackageId: objectPackage.id,
        name: note.name,
        description: ObjectDescription.of(note),
    });
    const rename = graph.Declaration.member(object, {
        kind: "method",
        name: "rename",
        description: {},
    });

    return {
        path: NOTE_MODULE,
        digest: await Digest.of(encode(NOTE_SOURCE), testHost().crypto),
        imports: [],
        exports: [],
        declarations: [object, rename],
        edges: [],
    };
}

/** Index the note module's code: the `Note` class and its `rename` method, by source range. */
async function notesCode(): Promise<graph.Document> {
    const range = (start: number, end: number) => ({ file: NOTE_MODULE, start, end });
    const rename = NOTE_SOURCE.indexOf("    rename");

    return {
        path: NOTE_MODULE,
        digest: await Digest.of(encode(NOTE_SOURCE), testHost().crypto),
        symbols: [
            {
                moniker: symbol("Note"),
                kind: "class",
                source: range(0, NOTE_SOURCE.length),
                signature: "class Note",
                isExported: true,
            },
            {
                moniker: symbol("Note.rename"),
                kind: "method",
                source: range(rename, NOTE_SOURCE.lastIndexOf("    }") + "    }".length),
                signature: "rename(title: string): string",
                isExported: true,
            },
        ],
        edges: [],
    };
}

/** Write the moniker of a symbol of the note module. */
function symbol(name: string): string {
    return graph.Moniker.of({ packageId: NOTES.id, module: NOTE_MODULE, name });
}
