import { dirname, join } from "node:path";
import type { FileSystem } from "@destack/environment";
import { graph, PackageId } from "@destack/package";
import { expect, refusal, test } from "@destack/test";
import { testSimulation } from "../test/world.ts";
import { Outliner } from "./outline.ts";

/** The package the outlined modules belong to. */
const PACKAGE = PackageId.parse("package-00000000-0000-7000-8000-00000000b001");

/** A module declaring a value and a type under one name, overloads, and accessors. */
const NOTE = `/** A note. */
export const Note = { title: "" };
/** The note's shape. */
export type Note = typeof Note;

export function pick(value: string): string;
export function pick(value: number): number;
export function pick(value: string | number) {
    return value;
}

export class Card {
    get title(): string {
        return "";
    }
    set title(value: string) {}
}
`;

/** A module re-exporting another and a JSON module, and calling a CommonJS dependency's member. */
const INDEX = `export * from "./note.ts";
export { default as tokens } from "./tokens.json";
import legacy = require("legacy");
export const run = () => legacy.start();
`;

/** A module registering tests through a namespace, an extended runner and a tagged table. */
const SUITE = `import * as vitest from "vitest";
import { test } from "@destack/test";
const it = test.extend({});
vitest.describe("notes", () => {
    it("opens", () => {});
    test.each\`value\`("reads %s", () => {});
});
`;

/** A module registering tests through conditional runners: bound, inline and from a factory. */
const CONDITIONAL = `import { test, testOn } from "@destack/test";
const onMac = test.skipIf(process.platform !== "darwin");
const onPostgresql = testOn("postgresql");
onMac("signs", () => {});
test.runIf(true)("opens", () => {});
onPostgresql("locks", () => {});
`;

/** A module defining an example outside a module-level constant. */
const STRAY = `import { defineExample } from "@destack/package/declare";
export default defineExample({ name: "stray" });
`;

/** The package's files, with a CommonJS dependency's declaration file. */
const FILES: Readonly<Record<string, string>> = {
    "package.json": JSON.stringify({ name: "@example/notes", type: "module" }),
    "destack.json": JSON.stringify({ id: PACKAGE }),
    "src/note.ts": NOTE,
    "src/index.ts": INDEX,
    "src/broken.ts": `export { missing } from "./note.ts";\n`,
    "src/tokens.json": JSON.stringify({ color: "violet" }),
    "src/stray.ts": STRAY,
    "tests/suite.test.ts": SUITE,
    "tests/conditional.test.ts": CONDITIONAL,
    "node_modules/legacy/package.json": JSON.stringify({ name: "legacy", types: "index.d.ts" }),
    "node_modules/legacy/index.d.ts": `declare namespace legacy {\n    function start(): void;\n}\nexport = legacy;\n`,
};

/** Write the package into a fresh directory, returning the directory and an outliner over it. */
async function outlined(files: FileSystem, environment: ConstructorParameters<typeof Outliner>[0]) {
    const directory = await files.makeTemporaryDirectory("outline-");
    for (const [path, text] of Object.entries(FILES)) {
        await files.mkdir(dirname(join(directory, path)), { recursive: true });
        await files.writeTextFile(join(directory, path), text);
    }

    return { directory, outliner: new Outliner(environment, new Map()) };
}

/** Name a symbol of the package by its module, name and member. */
const at = (module: string, name: string, member?: string) =>
    graph.Moniker.of({
        packageId: PACKAGE,
        module,
        name,
        ...(member === undefined ? {} : { member }),
    });

test("outline a value and a type under one name, overloads and accessors as one symbol each", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { directory, outliner } = await outlined(environment.files, environment);
    const outline = await outliner.outline(join(directory, "src/note.ts"));

    // keep each declaration of a name in source order, with the first documentation
    expect(
        outline.symbols.map((symbol) => [
            symbol.moniker,
            symbol.declarations.map((declaration) => [declaration.kind, declaration.signature]),
            symbol.comment,
            symbol.isExported,
        ]),
    ).toEqual([
        [
            at("src/note.ts", "Note"),
            [
                ["variable", "const Note"],
                ["typeAlias", "type Note = typeof Note"],
            ],
            "A note.",
            true,
        ],
        [
            at("src/note.ts", "pick"),
            [
                ["function", "function pick(value: string): string"],
                ["function", "function pick(value: number): number"],
                ["function", "function pick(value: string | number)"],
            ],
            undefined,
            true,
        ],
        [at("src/note.ts", "Card"), [["class", "class Card"]], undefined, true],
        [
            at("src/note.ts", "Card", "title"),
            [
                ["getter", "get title(): string"],
                ["setter", "set title(value: string)"],
            ],
            undefined,
            true,
        ],
    ]);

    // export the merged name as a value, as it declares one
    expect(outline.exports).toEqual([
        { name: "Note", symbol: at("src/note.ts", "Note"), isTypeOnly: false },
        { name: "pick", symbol: at("src/note.ts", "pick"), isTypeOnly: false },
        { name: "Card", symbol: at("src/note.ts", "Card"), isTypeOnly: false },
    ]);
});

test("resolve star and JSON re-exports and a CommonJS `export =` member, refusing an export that reaches nothing", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { directory, outliner } = await outlined(environment.files, environment);
    const outline = await outliner.outline(join(directory, "src/index.ts"));
    const legacy = graph.Moniker.of({
        packageId: "npm:legacy",
        module: "index.d.ts",
        name: "legacy",
        member: "start",
    });

    // export the star's names by their original symbols, and call the namespace's member
    expect({
        exports: outline.exports.map((each) => [each.name, each.symbol]),
        uses: outline.uses.map((use) => [use.kind, use.to]),
        broken: await refusal(outliner.outline(join(directory, "src/broken.ts"))),
    }).toEqual({
        exports: [
            ["Note", at("src/note.ts", "Note")],
            ["pick", at("src/note.ts", "pick")],
            ["Card", at("src/note.ts", "Card")],
            ["tokens", at("src/tokens.json", "default")],
            ["run", at("src/index.ts", "run")],
        ],
        uses: [["calls", legacy]],
        broken: [
            "BAD_REQUEST",
            "INSPECTION_INVALID",
            "export 'missing' of 'src/broken.ts' is invalid: it reaches no declaration",
        ],
    });
});

test("read tests through a namespace import, an extended runner and a tagged table", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { directory, outliner } = await outlined(environment.files, environment);
    const { tests } = await outliner.outline(join(directory, "tests/suite.test.ts"));

    expect(tests.map((each) => [each.kind, each.name, each.suites, each.modifiers])).toEqual([
        ["suite", "notes", [], []],
        ["test", "opens", ["notes"], []],
        ["test", "reads %s", ["notes"], ["each"]],
    ]);
});

test("read tests through conditional runners, recording each condition, and never a runner's making", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { directory, outliner } = await outlined(environment.files, environment);
    const { tests } = await outliner.outline(join(directory, "tests/conditional.test.ts"));

    expect(tests.map((each) => [each.kind, each.name, each.modifiers])).toEqual([
        ["test", "signs", ["skipIf"]],
        ["test", "opens", ["runIf"]],
        ["test", "locks", ["skipIf"]],
    ]);
});

test("refuse an example defined outside a module-level constant", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { directory, outliner } = await outlined(environment.files, environment);
    const offset = STRAY.indexOf("defineExample({");

    expect(await refusal(outliner.outline(join(directory, "src/stray.ts")))).toEqual([
        "BAD_REQUEST",
        "INSPECTION_INVALID",
        `definition example at 'src/stray.ts:${String(offset)}' is invalid: it must initialize a module-level constant`,
    ]);
});
