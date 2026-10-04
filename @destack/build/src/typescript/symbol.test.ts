import { expect, test } from "@destack/test";
import { mkdir, mkdtemp, realpath, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { PackageBuilder } from "../build/builder.ts";

/** The workspace's packages: an app declaring a document, and a sibling dependency augmenting it. */
const FILES: Readonly<Record<string, string>> = {
    "bun.lock": JSON.stringify({ lockfileVersion: 1, workspaces: {}, packages: {} }),
    "app/destack.json": JSON.stringify({
        $schema: "https://destack.app/schemas/2026.9.0/destack.json",
        id: "package-01996ab0-0000-7000-8000-00000000000c",
        language: "typescript",
        runtimes: ["bun"],
    }),
    "app/package.json": JSON.stringify({
        name: "@example/app",
        version: "2026.9.0",
        type: "module",
        exports: { ".": "./src/index.ts" },
        dependencies: { "@example/other": "workspace:*" },
    }),
    "app/src/index.ts":
        'import "@example/other";\n\n/** The fields of a document. */\nexport interface Document {\n    /** The title. */\n    readonly title: string;\n}\n',
    "other/package.json": JSON.stringify({
        name: "@example/other",
        version: "2026.9.0",
        type: "module",
        exports: { ".": "./index.ts" },
    }),
    "other/index.ts":
        'declare module "../app/src/index.ts" {\n    /** The fields the other package adds. */\n    interface Document {\n        /** The tag. */\n        readonly tag?: string;\n    }\n}\n\nexport {};\n',
};

test("describe a symbol by the package's declarations, leaving a linked dependency's augmentation", async () => {
    // link the dependency beside the app, as a workspace installs it
    const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-augmentation-")));
    try {
        for (const [path, text] of Object.entries(FILES)) {
            await mkdir(dirname(join(directory, path)), { recursive: true });
            await writeFile(join(directory, path), text);
        }
        await mkdir(join(directory, "app/node_modules/@example"), { recursive: true });
        await symlink(join(directory, "other"), join(directory, "app/node_modules/@example/other"));

        // inspect the app
        await using builder = await PackageBuilder.start(join(directory, "app"));
        const inspection = await builder.inspect({ runtime: "bun" });
        const symbols = inspection.code.flatMap((module) => module.symbols);

        // describe the document by its one declaration and member in the app
        expect(
            symbols.map((symbol) => ({
                name: symbol.name,
                declarations: symbol.declarations.map((declaration) => declaration.source),
                members: symbol.members.map((member) => member.name),
            })),
        ).toEqual([
            {
                name: "Document",
                declarations: [{ file: "src/index.ts", start: 59, end: 138 }],
                members: ["title"],
            },
        ]);
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
});
