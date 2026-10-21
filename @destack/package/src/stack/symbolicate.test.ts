import { expect, test } from "@destack/test";
import * as graph from "../graph/index.ts";
import { StackFrame } from "./frame.ts";
import { Symbolicator } from "./symbolicate.ts";

/** The note module's source: a class with a rename method, and a helper outside it. */
const SOURCE = [
    "export class Note {",
    "    rename(title: string) {",
    "        return check(title);",
    "    }",
    "}",
    "function check(title: string) {",
    "    return title;",
    "}",
].join("\n");

/** The note module's package and moniker prefix. */
const AT = "package-01996ab0-0000-7000-8000-000000000001/src/note.ts";

/** Offset of a source line's start. */
const lineAt = (line: number) =>
    SOURCE.split("\n")
        .slice(0, line - 1)
        .join("\n").length + (line > 1 ? 1 : 0);

/** The note module's graph. */
const module = graph.Module.parse({
    path: "src/note.ts",
    digest: "0000000000000000000000000000000000000000000000000000000000000000",
    imports: [],
    exports: [],
    declarations: [
        {
            moniker: `${AT}#Note:object`,
            symbol: `${AT}#Note`,
            kind: "object",
            kindPackageId: "package-01996ab0-0000-7000-8000-000000000002",
            name: "note",
            description: {},
        },
        {
            moniker: `${AT}#Note.rename:method`,
            symbol: `${AT}#Note`,
            kind: "method",
            kindPackageId: "package-01996ab0-0000-7000-8000-000000000002",
            name: "rename",
            description: {},
        },
    ],
    edges: [],
});

/** The note module's code, as the code index keeps it. */
const code = graph.Document.parse({
    path: "src/note.ts",
    digest: "0000000000000000000000000000000000000000000000000000000000000000",
    symbols: [
        {
            moniker: `${AT}#Note`,
            kind: "class",
            source: { file: "src/note.ts", start: 0, end: lineAt(6) - 1 },
            signature: "class Note",
            isExported: true,
        },
        {
            moniker: `${AT}#Note.rename`,
            kind: "method",
            source: { file: "src/note.ts", start: lineAt(2), end: lineAt(5) - 1 },
            signature: "rename(title)",
            isExported: true,
        },
        {
            moniker: `${AT}#check`,
            kind: "function",
            source: { file: "src/note.ts", start: lineAt(6), end: SOURCE.length },
            signature: "check(title)",
            isExported: false,
        },
    ],
    edges: [],
});

/** The workload's source map. */
const MAP = JSON.stringify({
    version: 3,
    sources: ["../../src/note.ts", "../../node_modules/@example/pad/index.js"],
    names: [],
    // generated 1:0 → note.ts 3:8; 2:0 → note.ts 7:4; 3:0 → pad index.js 1:0
    mappings: "AAEQ;AAIJ;ACNJ",
});

/** A build whose workload maps into the note module. */
const build = {
    sourceMaps: async () => [
        { generated: "output/bun/workload.js", map: "output/bun/workload.js.map" },
    ],
    load: async (path: string) => new TextEncoder().encode(path.endsWith(".map") ? MAP : SOURCE),
};

test("map frames to their sources and resolve in-app ones to their innermost symbol and the declaration it is or belongs to", async () => {
    // map the frames through the build's source map, then attribute them through the module's code
    const located = { code, module, lines: StackFrame.lineStarts(SOURCE) };
    const frames = await new Symbolicator(build).frames(
        [1, 2, 3].map((line) => ({
            function: "a",
            file: "file:///srv/output/bun/workload.js",
            line,
            column: 1,
            isInApp: true,
        })),
    );
    const elsewhere = await new Symbolicator(build).frames([
        { function: "c", file: "file:///srv/elsewhere.js", line: 1, column: 1, isInApp: true },
    ]);

    // name the method's declaration, the helper without one, and leave the dependency's and an unmapped file's frames out of the app
    const attributed = [...frames, ...elsewhere].map((frame) =>
        StackFrame.attribute(frame, located),
    );
    expect(
        attributed.map(({ path, line, column, isInApp, moniker, declaration }) => ({
            path,
            line,
            column,
            isInApp,
            moniker: moniker?.slice(AT.length),
            declaration: declaration?.name,
        })),
    ).toEqual([
        {
            path: "src/note.ts",
            line: 3,
            column: 9,
            isInApp: true,
            moniker: "#Note.rename",
            declaration: "rename",
        },
        {
            path: "src/note.ts",
            line: 7,
            column: 5,
            isInApp: true,
            moniker: "#check",
            declaration: undefined,
        },
        {
            path: "node_modules/@example/pad/index.js",
            line: 1,
            column: 1,
            isInApp: false,
            moniker: undefined,
            declaration: undefined,
        },
        {
            path: undefined,
            line: 1,
            column: 1,
            isInApp: false,
            moniker: undefined,
            declaration: undefined,
        },
    ]);
});

test("leave a frame unattributed outside the module, past its lines, outside any symbol, or out of the app", () => {
    // place frames in another module, past the last line, on the blank line between symbols, and in a dependency
    const located = { code, module, lines: StackFrame.lineStarts(SOURCE) };
    const frame = {
        function: "a",
        file: "workload.js",
        line: 3,
        column: 9,
        isInApp: true,
        path: "src/note.ts",
    };
    const frames = [
        { ...frame, path: "src/other.ts" },
        { ...frame, line: 40 },
        { ...frame, line: 5, column: 2 },
        { ...frame, isInApp: false },
    ];

    expect(frames.map((each) => StackFrame.attribute(each, located))).toEqual(frames);
});
