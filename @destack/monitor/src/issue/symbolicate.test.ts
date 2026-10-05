import { expect, test } from "@destack/test";
import { graph } from "@destack/package";
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

/** The note module's graph: the class, its method and the helper, the object type at the class with its method member. */
const module = graph.Module.parse({
    path: "src/note.ts",
    digest: "0000000000000000000000000000000000000000000000000000000000000000",
    imports: [],
    exports: [],
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
    declarations: [
        {
            moniker: `${AT}#Note:object`,
            symbol: `${AT}#Note`,
            kind: "object",
            package: "package-01996ab0-0000-7000-8000-000000000002",
            name: "note",
            description: {},
        },
        {
            moniker: `${AT}#Note.rename:method`,
            symbol: `${AT}#Note`,
            kind: "method",
            package: "package-01996ab0-0000-7000-8000-000000000002",
            name: "rename",
            description: {},
        },
    ],
    edges: [],
});

/** The workload's source map: line 1 into `rename`, line 2 into `check`, line 3 into a dependency. */
const MAP = JSON.stringify({
    version: 3,
    sources: ["../../src/note.ts", "../../node_modules/@example/pad/index.js"],
    names: [],
    // generated 1:0 → note.ts 3:8; 2:0 → note.ts 7:4; 3:0 → pad index.js 1:0
    mappings: "AAEQ;AAIJ;ACNJ",
});

/** A build whose workload maps into the note module, which its graph and retained source describe. */
const build = {
    sourceMaps: async () => [
        { generated: "output/bun/workload.js", map: "output/bun/workload.js.map" },
    ],
    graph: async () => ({ modules: { "src/note.ts": module.digest } }),
    module: async () => module,
    load: async (path: string) => new TextEncoder().encode(path.endsWith(".map") ? MAP : SOURCE),
};

test("map frames to their sources and resolve in-app ones to their innermost symbol and the declaration it is or belongs to", async () => {
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
    expect(
        [...frames, ...elsewhere].map(({ path, line, column, isInApp, moniker, declaration }) => ({
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
