import { expect, test } from "@destack/test";
import { graph, Package, type DeclarationDescription } from "@destack/package";
import { DeclarationGraph } from "./declaration.ts";

/** The package declaring the declarations. */
const app = Package.parse({
    id: "package-01996ab0-0000-7000-8000-00000000000a",
    name: "@example/app",
    version: "2026.9.0",
});

/** A dependency of the app declaring an object type. */
const library = Package.parse({
    id: "package-01996ab0-0000-7000-8000-00000000000b",
    name: "@example/library",
    version: "2026.9.0",
});

/** The package declaring the kinds. */
const kinds = Package.parse({
    id: "package-01996ab0-0000-7000-8000-00000000000c",
    name: "@example/kinds",
    version: "2026.9.0",
});

/** Describe a declaration of a kind at an export of a package, with the symbols its kind lists. */
function declare(
    owner: Package,
    kind: string,
    module: string,
    symbol: string,
    name: string,
    symbols: graph.MemberSymbol[],
): DeclarationDescription {
    return {
        name,
        kind,
        package: kinds,
        constructor: { package: kinds, symbol: { module: "src/index.ts", name: "define" } },
        symbol: { package: owner, symbol: { module, name: symbol } },
        source: { file: module, line: 0, column: 0 },
        description: { name },
        symbols,
    };
}

/** Name a moniker of the app. */
const at = (path: string) => `${app.id}/${path}`;

test("derive the member symbols kinds list and resolve their relationships by kind and name", () => {
    // declare an object type with two methods on a table, a database holding it that two outputs evaluate, and a view and command on it
    const declarations = new DeclarationGraph(app, [
        declare(app, "object", "src/note.ts", "Note", "note", [
            {
                member: { kind: "method", name: "get", description: { mutates: false } },
                relationships: [{ kind: "reads", symbol: { kind: "table", name: "app__note" } }],
            },
            {
                member: { kind: "method", name: "rename", description: { mutates: true } },
                relationships: [{ kind: "writes", symbol: { kind: "table", name: "app__note" } }],
            },
        ]),
        declare(app, "resource", "src/database.ts", "database", "main", [
            {
                member: { kind: "table", name: "app__note", description: { sqlite: {} } },
                relationships: [],
            },
        ]),
        declare(app, "resource", "src/database.ts", "database", "main", [
            {
                member: { kind: "table", name: "app__note", description: { sqlite: {} } },
                relationships: [],
            },
        ]),
        declare(app, "view", "src/view.ts", "editor", "editor", [
            {
                relationships: [
                    {
                        kind: "presents",
                        symbol: { packageId: app.id, kind: "object", name: "note" },
                    },
                    {
                        kind: "presents",
                        symbol: { packageId: library.id, kind: "object", name: "tag" },
                    },
                ],
            },
        ]),
        declare(app, "command", "src/view.ts", "rename", "rename", [
            {
                relationships: [
                    {
                        kind: "invokes",
                        symbol: {
                            kind: "method",
                            name: "rename",
                            parent: { kind: "object", name: "note" },
                        },
                    },
                    {
                        kind: "invokes",
                        symbol: {
                            kind: "method",
                            name: "rename",
                            parent: { kind: "object", name: "tag" },
                        },
                    },
                ],
            },
        ]),
        declare(library, "object", "src/tag.ts", "Tag", "tag", [
            {
                member: { kind: "method", name: "rename", description: { mutates: true } },
                relationships: [],
            },
        ]),
    ]);

    // store the app's declarations with their members, and resolve edges in the build, the library's tag too
    const described = (path: string) =>
        declarations
            .describe(path)
            .map(({ moniker, symbol, kind, name, description }) => [
                moniker,
                symbol,
                kind,
                name,
                description,
            ]);
    expect(
        ["src/note.ts", "src/database.ts", "src/view.ts", "src/tag.ts"].map((path) => [
            described(path),
            declarations.edges(path),
        ]),
    ).toEqual([
        [
            [
                [
                    at("src/note.ts#Note:object:note"),
                    at("src/note.ts#Note"),
                    "object",
                    "note",
                    { name: "note" },
                ],
                [
                    at("src/note.ts#Note:object:note.get:method"),
                    at("src/note.ts#Note"),
                    "method",
                    "get",
                    { mutates: false },
                ],
                [
                    at("src/note.ts#Note:object:note.rename:method"),
                    at("src/note.ts#Note"),
                    "method",
                    "rename",
                    { mutates: true },
                ],
            ],
            [
                {
                    from: at("src/note.ts#Note:object:note.get:method"),
                    to: at("src/database.ts#database:resource:main.app__note:table"),
                    kind: "reads",
                },
                {
                    from: at("src/note.ts#Note:object:note.rename:method"),
                    to: at("src/database.ts#database:resource:main.app__note:table"),
                    kind: "writes",
                },
            ],
        ],
        [
            [
                [
                    at("src/database.ts#database:resource:main"),
                    at("src/database.ts#database"),
                    "resource",
                    "main",
                    { name: "main" },
                ],
                [
                    at("src/database.ts#database:resource:main.app__note:table"),
                    at("src/database.ts#database"),
                    "table",
                    "app__note",
                    { sqlite: {} },
                ],
            ],
            [],
        ],
        [
            [
                [
                    at("src/view.ts#editor:view:editor"),
                    at("src/view.ts#editor"),
                    "view",
                    "editor",
                    { name: "editor" },
                ],
                [
                    at("src/view.ts#rename:command:rename"),
                    at("src/view.ts#rename"),
                    "command",
                    "rename",
                    { name: "rename" },
                ],
            ],
            [
                {
                    from: at("src/view.ts#editor:view:editor"),
                    to: at("src/note.ts#Note:object:note"),
                    kind: "presents",
                },
                {
                    from: at("src/view.ts#editor:view:editor"),
                    to: `${library.id}/src/tag.ts#Tag:object:tag`,
                    kind: "presents",
                },
                {
                    from: at("src/view.ts#rename:command:rename"),
                    to: at("src/note.ts#Note:object:note.rename:method"),
                    kind: "invokes",
                },
            ],
        ],
        [[], []],
    ]);
});
