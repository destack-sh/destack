import { testSimulation } from "../test/world.ts";
import { CompilerPool } from "../index.ts";
import { expect, test } from "@destack/test";
import { dirname, join } from "node:path";
import { Workspace } from "../dependency/workspace.ts";
import { LocalPackageStore } from "../store/local.ts";

/** The workspace's packages: an app declaring a document, and a dependency augmenting it. */
const FILES: Readonly<Record<string, string>> = {
    "bun.lock": JSON.stringify({
        lockfileVersion: 2,
        workspaces: { app: { name: "@example/app" }, other: { name: "@example/other" } },
        packages: {},
    }),
    ".gitignore": "node_modules/\n",
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
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.realPath(
        await files.makeTemporaryDirectory("destack-augmentation-"),
    );
    try {
        for (const [path, text] of Object.entries(FILES)) {
            await files.mkdir(dirname(join(directory, path)), { recursive: true });
            await files.writeTextFile(join(directory, path), text);
        }
        await files.mkdir(join(directory, "app/node_modules/@example"), { recursive: true });
        await files.symlink(
            join(directory, "other"),
            join(directory, "app/node_modules/@example/other"),
        );
        expect(await environment.process.spawn(["git", "init", "--quiet", directory]).exited).toBe(
            0,
        );

        // inspect the app
        await using store = await LocalPackageStore.temporary(environment);
        await using compilers = await CompilerPool.open(store);
        const workspace = await Workspace.open(join(directory, "app"), compilers);
        const inspection = await workspace.inspect({ runtime: "bun" });
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
        await files.remove(directory, { recursive: true });
    }
});

test("describe a class's private members by the names their declarations write", async () => {
    // write a package with a class with a private field and method
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.realPath(await files.makeTemporaryDirectory("destack-private-"));
    try {
        await files.writeTextFile(
            join(directory, "bun.lock"),
            JSON.stringify({
                lockfileVersion: 2,
                workspaces: { app: { name: "@example/app" } },
                packages: {},
            }),
        );
        await files.mkdir(join(directory, "app/src"), { recursive: true });
        await files.writeTextFile(
            join(directory, "app/destack.json"),
            FILES["app/destack.json"] ?? "",
        );
        await files.writeTextFile(
            join(directory, "app/package.json"),
            JSON.stringify({ ...JSON.parse(FILES["app/package.json"] ?? "{}"), dependencies: {} }),
        );
        await files.writeTextFile(
            join(directory, "app/src/index.ts"),
            "/** An inbox. */\nexport class Inbox {\n    /** The unread count. */\n    #unread = 0;\n\n    /** Read every message. */\n    read(): number {\n        return this.#clear();\n    }\n\n    /** Clear the unread count. */\n    #clear(): number {\n        this.#unread = 0;\n\n        return this.#unread;\n    }\n}\n",
        );
        expect(await environment.process.spawn(["git", "init", "--quiet", directory]).exited).toBe(
            0,
        );

        // inspect the app
        await using store = await LocalPackageStore.temporary(environment);
        await using compilers = await CompilerPool.open(store);
        const workspace = await Workspace.open(join(directory, "app"), compilers);
        const inspection = await workspace.inspect({ runtime: "bun" });
        const symbols = inspection.code.flatMap((module) => module.symbols);

        // name the private members, and the private method the public one calls, without the compiler's per-program prefix
        expect(
            symbols.map((symbol) => [symbol.name, symbol.members.map((member) => member.name)]),
        ).toEqual([
            ["Inbox", ["#unread", "read", "#clear"]],
            ["Inbox.#clear", []],
        ]);
    } finally {
        await files.remove(directory, { recursive: true });
    }
});
