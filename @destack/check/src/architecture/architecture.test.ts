import { expect, test } from "@destack/test";
import { Architecture, type ArchitecturePackage } from "./architecture.ts";

/** An architecture of two layers: views above the core. */
const TEXT = [
    "# Architecture",
    "",
    "## 01 View",
    "",
    "| Package | What it does | Prior art |",
    "|---|---|---|",
    "| [`@destack/ui`](@destack/ui) | Build views. | shadcn/ui |",
    "",
    "## 02 Core",
    "",
    "| Package | What it does | Prior art |",
    "|---|---|---|",
    "| [`@destack/schema`](@destack/schema) | Declare schemas. | Zod |",
    "| [`@destack/web`](@destack/web) | Build websites. | Next.js |",
    "",
].join("\n");

test("refuse an unlisted package, a removed one, a different description and a dependency on a higher layer", () => {
    // describe a workspace whose core depends on the views, with one package no layer lists
    const packages = new Map<string, ArchitecturePackage>([
        ["@destack/ui", { description: "Build views.", dependencies: ["@destack/schema"] }],
        [
            "@destack/schema",
            { description: "Declare schemas and convert them.", dependencies: ["@destack/ui"] },
        ],
        ["@destack/icon", { description: "Draw icons.", dependencies: [] }],
    ]);

    // expect each rule's message at its row, the unlisted package at the top
    const diagnostics = Architecture.parse(TEXT).check(packages, "ARCHITECTURE.md", TEXT);
    expect(diagnostics.map((each) => [each.code, each.labels[0]?.span.line, each.message])).toEqual(
        [
            [
                "architecture(description)",
                13,
                "describe '@destack/schema' as its package.json does: 'Declare schemas and convert them.'",
            ],
            [
                "architecture(member)",
                14,
                "remove '@destack/web': the workspace has no such package",
            ],
            ["architecture(member)", 1, "list '@destack/icon' in its layer"],
            [
                "architecture(layer)",
                13,
                "'@destack/schema' in Core depends on '@destack/ui' in the higher layer View",
            ],
        ],
    );
});

test("accept every package listed once, as described, depending downward", () => {
    const packages = new Map<string, ArchitecturePackage>([
        ["@destack/ui", { description: "Build views.", dependencies: ["@destack/schema"] }],
        ["@destack/schema", { description: "Declare schemas.", dependencies: [] }],
        ["@destack/web", { description: "Build websites.", dependencies: [] }],
    ]);

    expect(Architecture.parse(TEXT).check(packages, "ARCHITECTURE.md", TEXT)).toEqual([]);
});
