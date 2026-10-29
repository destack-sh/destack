import { testSimulation } from "@destack/test/world";
import { Digest } from "@destack/schema";
import { expect, test } from "@destack/test";
import { PackageId } from "../definition/package.ts";
import { Moniker } from "../graph/moniker.ts";
import { MemoryBuild } from "../test/memory.ts";
import { Attribution } from "./attribution.ts";

/** The package the note module belongs to. */
const PACKAGE_ID = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000a1");

/** The note module's source: a class whose rename method fails. */
const SOURCE = [
    "export class Note {",
    "    rename() {",
    "        return fail();",
    "    }",
    "}",
].join("\n");

/** A source range of the note module. */
function range(start: number, end: number) {
    return { file: "src/note.ts", start, end };
}

/** Key a symbol of the note module. */
function at(name: string): Moniker {
    return Moniker.of({ packageId: PACKAGE_ID, module: "src/note.ts", name });
}

test("attribute an in-app frame to its innermost symbol and its declaration, leaving other frames", async () => {
    // build the note module with its rename method's declaration, and index its code
    await using simulation = testSimulation();
    const { crypto } = simulation.environment("test");
    const bytes = new TextEncoder().encode(SOURCE);
    const digest = await Digest.of(bytes, crypto);
    const owner = { id: PACKAGE_ID, name: "@example/note", version: "2026.10.0" };
    const build = await MemoryBuild.write(
        new Map([["src/note.ts", bytes]]),
        { package: owner },
        crypto,
    );
    const modules = [
        {
            path: "src/note.ts",
            digest,
            imports: [],
            exports: [],
            declarations: [
                {
                    moniker: `${at("Note.rename")}:method`,
                    symbol: at("Note"),
                    kind: "method",
                    kindPackageId: PACKAGE_ID,
                    name: "rename",
                    description: {},
                },
            ],
            edges: [],
        },
    ];
    const rename = SOURCE.indexOf("    rename");
    const attribution = new Attribution(build, {
        modules,
        code: [
            {
                path: "src/note.ts",
                digest,
                symbols: [
                    {
                        moniker: at("Note"),
                        kind: "class",
                        source: range(0, SOURCE.length),
                        signature: "class Note",
                        isExported: true,
                    },
                    {
                        moniker: at("Note.rename"),
                        kind: "method",
                        source: range(rename, SOURCE.lastIndexOf("    }") + 5),
                        signature: "rename(): never",
                        isExported: true,
                    },
                ],
                edges: [],
            },
        ],
    });

    // attribute a frame inside rename, one in a module the index lacks, and a dependency's
    const inside = { file: "workload.js", line: 3, column: 9, isInApp: true, path: "src/note.ts" };
    const elsewhere = { ...inside, path: "src/other.ts" };
    const dependency = { file: "node_modules/x/index.js", line: 1, column: 1, isInApp: false };
    expect(await attribution.frames([inside, elsewhere, dependency])).toEqual([
        {
            ...inside,
            moniker: at("Note.rename"),
            declaration: { moniker: `${at("Note.rename")}:method`, kind: "method", name: "rename" },
        },
        elsewhere,
        dependency,
    ]);
});
