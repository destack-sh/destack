import { DeclarationDescription } from "@destack/package";
import { expect, test } from "@destack/test";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { schema } from "@destack/schema";
import { TestDeclaration } from "./test.ts";
import { Compiler } from "../build/compiler.ts";

/** The packages of the inspected workspace, by identity. */
const packages = {
    owner: { id: "package-01996ab0-0000-7000-8000-00000000000a", name: "@example/owner" },
    other: { id: "package-01996ab0-0000-7000-8000-00000000000b", name: "@example/other" },
    app: { id: "package-01996ab0-0000-7000-8000-00000000000c", name: "@example/app" },
};

/** The app module declaring the greeting. */
const GREETING =
    'import { defineGreeting } from "@example/other";\n\nexport const greeting = defineGreeting({ name: "greeting" });\n';

/** Write a workspace whose app declares a greeting, as its own kind and the owner's kind. */
async function writeWorkspace(directory: string, owner: string, app = GREETING): Promise<void> {
    // write each package's constructor, the function describing a declaration by its name, and the function reading a greeting as a thing
    const describe =
        "export function describe(value: { name: string }) {\n    return { name: value.name };\n}\nexport function thing(value: { name: string }) {\n    return value;\n}\n";
    const define =
        "export function define(definition: { name: string }, module?: { package: object }) {\n    return { ...definition, package: module!.package, renamed(name: string) { return { ...this, name }; } };\n}\n";
    const files: Record<string, string> = {
        "node_modules/@example/owner/index.ts": define.replace("define(", "defineThing("),
        "node_modules/@example/owner/inspect.ts": describe,
        "node_modules/@example/other/index.ts": define.replace("define(", "defineGreeting("),
        "node_modules/@example/other/inspect.ts": describe,
        "src/index.ts": app,
    };
    for (const [key, identity] of Object.entries(packages)) {
        const root = key === "app" ? "" : `node_modules/${identity.name}/`;
        const constructors =
            key === "owner"
                ? {
                      defineThing: {
                          describes: [{ kind: "thing", function: "./inspect#describe" }],
                      },
                  }
                : key === "other"
                  ? {
                        defineGreeting: {
                            describes: [
                                { kind: "greeting", function: "./inspect#describe" },
                                { kind: "thing", package: owner, value: "./inspect#thing" },
                            ],
                        },
                    }
                  : {};
        const stamps = Object.fromEntries(
            Object.keys(constructors).map((name) => [name, { module: 1 }]),
        );
        files[`${root}destack.json`] = JSON.stringify({
            $schema: "https://destack.app/schemas/2026.9.0/destack.json",
            id: identity.id,
            language: "typescript",
            runtimes: ["bun"],
            declarations: constructors,
            stamps,
        });
        files[`${root}package.json`] = JSON.stringify({
            name: identity.name,
            version: "2026.9.0",
            type: "module",
            exports:
                key === "app"
                    ? { ".": "./src/index.ts" }
                    : { ".": "./index.ts", "./inspect": "./inspect.ts" },
            dependencies:
                key === "app" ? { "@example/other": "2026.9.0" } : { "@example/owner": "2026.9.0" },
        });
    }
    for (const [path, text] of Object.entries(files)) {
        await mkdir(dirname(join(directory, path)), { recursive: true });
        await writeFile(join(directory, path), text);
    }
}

test("describe one declaration as every kind its constructor lists, each owned by the package declaring the kind", async () => {
    // inspect the app's greeting
    const directory = await mkdtemp(join(tmpdir(), "destack-kinds-"));
    try {
        await writeWorkspace(directory, "@example/owner");
        await using builder = await Compiler.start(directory);
        const inspection = await builder.inspect({ runtime: "bun" });
        const { declarations, tests } = schema
            .object({
                tests: schema.array(TestDeclaration),
                declarations: schema.array(DeclarationDescription),
            })
            .parse(inspection.descriptions);
        expect(tests).toEqual([]);

        // describe the greeting as the other package's kind and as the owner's kind
        const version = { version: "2026.9.0" };
        expect(
            declarations.map((declaration) => [
                declaration.name,
                declaration.kind,
                declaration.package,
                declaration.constructor.package,
                declaration.description,
            ]),
        ).toEqual([
            [
                "greeting",
                "greeting",
                { ...packages.other, ...version },
                { ...packages.other, ...version },
                { name: "greeting" },
            ],
            [
                "greeting",
                "thing",
                { ...packages.owner, ...version },
                { ...packages.other, ...version },
                { name: "greeting" },
            ],
        ]);
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
});

test("describe a declaration a method call chained on its constructor returns, as a type implemented in place", async () => {
    // inspect the app's greeting, renamed by a call on the declaration
    const directory = await mkdtemp(join(tmpdir(), "destack-chained-"));
    try {
        await writeWorkspace(
            directory,
            "@example/owner",
            GREETING.replace(
                'defineGreeting({ name: "greeting" })',
                'defineGreeting({ name: "greeting" }).renamed("hello")',
            ),
        );
        await using builder = await Compiler.start(directory);
        const inspection = await builder.inspect({ runtime: "bun" });
        const { declarations } = schema
            .object({
                tests: schema.array(TestDeclaration),
                declarations: schema.array(DeclarationDescription),
            })
            .parse(inspection.descriptions);

        // find the exported constant, and describe and name it as the value the chained call returns, once per kind
        expect(
            declarations.map((declaration) => [
                declaration.kind,
                declaration.name,
                declaration.description,
            ]),
        ).toEqual([
            ["greeting", "hello", { name: "hello" }],
            ["thing", "hello", { name: "hello" }],
        ]);
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
});

test("describe a module's default-exported declaration under the default export", async () => {
    // inspect the app's greeting, exported as the module's default
    const directory = await mkdtemp(join(tmpdir(), "destack-default-"));
    try {
        await writeWorkspace(
            directory,
            "@example/owner",
            'import { defineGreeting } from "@example/other";\n\nexport default defineGreeting({ name: "greeting" });\n',
        );
        await using builder = await Compiler.start(directory);
        const inspection = await builder.inspect({ runtime: "bun" });
        const { declarations } = schema
            .object({
                tests: schema.array(TestDeclaration),
                declarations: schema.array(DeclarationDescription),
            })
            .parse(inspection.descriptions);

        // describe the greeting once as its own kind and once as the owner's, its symbol the module's default export
        expect(
            declarations.map((declaration) => [
                declaration.kind,
                declaration.name,
                declaration.symbol.symbol.name,
                declaration.description,
            ]),
        ).toEqual([
            ["greeting", "greeting", "default", { name: "greeting" }],
            ["thing", "greeting", "default", { name: "greeting" }],
        ]);
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
});
