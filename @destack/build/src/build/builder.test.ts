import { expect, test } from "@destack/test";
import { cp, mkdir, mkdtemp, readFile, realpath, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { buildPackage, PackageBuild, PackageBuilder, readDependencies } from "../index.ts";
import type { PackageManifest } from "@destack/package/manifest";
import { request } from "../../tests/fixture/library/request.ts";
import * as resource from "../../tests/fixture/resource/request.ts";
import {
    Fixture,
    expectFiles,
    importBuilt,
    isNoteModule,
    type NoteModule,
} from "../../tests/fixture.ts";
import { requests } from "../../tests/fixture/web/request.ts";
import * as service from "../../tests/fixture/service/request.ts";
import { formatSource } from "@destack/check";
import { Vocabulary } from "@destack/resource";
import { present, schema } from "@destack/schema";
import { LocalBucket } from "@destack/bucket/local";
import { PackageStore } from "../store/index.ts";

/** The error a denied read reports: Seatbelt refuses it, bubblewrap hides the file. */
const DENIED_READ: Partial<Record<NodeJS.Platform, string>> = { darwin: "EPERM", linux: "ENOENT" };

/** The tables of a database resource's description, by dialect. */
const TableDescription = schema.looseObject({
    tables: schema.record(
        schema.string(),
        schema.array(
            schema.looseObject({
                table: schema.looseObject({
                    columns: schema.array(schema.looseObject({ name: schema.string() })),
                }),
            }),
        ),
    ),
});
/** The tables of a database resource's description, by dialect. */
type TableDescription = schema.Infer<typeof TableDescription>;

test("rebuild a web application with emitted assets and restore every output", async () => {
    await using fixture = await Fixture.open("web");
    const file = join(fixture.source, "src/app.tsx");
    const original = await readFile(file, "utf8");

    // declare the asset imports used by this standalone application
    const declaration = join(fixture.source, "src/assets.d.ts");
    await writeFile(
        declaration,
        await formatSource(
            declaration,
            '/** SVG assets imported as URLs. */\ndeclare module "*.svg?url" { const url: string; export default url; }',
        ),
    );

    // retain two distinct emitted assets through the application's public render
    for (const name of ["first", "second"]) {
        await writeFile(
            join(fixture.source, `src/${name}.svg`),
            `<svg xmlns="http://www.w3.org/2000/svg"><!--${name.repeat(1500)}--></svg>`,
        );
    }
    const source = await formatSource(
        file,
        'import first from "./first.svg?url";\nimport second from "./second.svg?url";\n' +
            original.replace(
                '<main style={theme.variables("light", DEFAULT_PREFERENCES)}>',
                '<main style={theme.variables("light", DEFAULT_PREFERENCES)}><img src={first} /><img src={second} />',
            ),
    );
    await writeFile(file, source);
    await using builder = await PackageBuilder.start(fixture.source);
    const options = { dependencies: fixture.dependencies, outputs: { website: requests.browser } };

    // compare the full distribution before and after an application edit
    await using first = await builder.build(options);
    await using repeated = await builder.build(options);
    expect(repeated.manifest).toEqual(first.manifest);
    await expectFiles(repeated, first);
    await writeFile(file, source.replace("Hello Destack", "Edited Destack"));
    await using edited = await builder.build(options);
    expect(edited.manifest).not.toEqual(first.manifest);
    await writeFile(file, source);
    await using restored = await builder.build(options);
    expect(restored.manifest).toEqual(first.manifest);
    await expectFiles(restored, first);
});

test("reinspect edited declaration helpers in a retained compiler", async () => {
    const fixture = new URL("../../tests/fixture/resource/source/", import.meta.url);
    const directory = await mkdtemp(join(tmpdir(), "destack-declaration-edit-"));
    try {
        // copy the fixture beside its installed dependencies
        await cp(fixture, directory, {
            recursive: true,
            filter: (path) => !path.endsWith("/node_modules") && !path.endsWith("\\node_modules"),
        });
        const modules = join(directory, "node_modules");
        await mkdir(join(modules, "@example/noisy"), { recursive: true });
        await symlink(
            fileURLToPath(new URL("node_modules/@destack", fixture)),
            join(modules, "@destack"),
            "junction",
        );

        // declare a dependency that logs to the console when called
        await writeFile(
            join(modules, "@example/noisy/package.json"),
            '{ "name": "@example/noisy", "type": "module", "exports": "./index.js" }\n',
        );
        await writeFile(
            join(modules, "@example/noisy/index.js"),
            'export function noisy(name) {\n    console.info("inspecting", name);\n    return name;\n}\n',
        );
        await writeFile(
            join(modules, "@example/noisy/index.d.ts"),
            "/** Log a name and return it. */\nexport declare function noisy(name: string): string;\n",
        );
        const manifest = join(directory, "package.json");
        await writeFile(
            manifest,
            (await readFile(manifest, "utf8")).replace(
                '"@destack/db": "workspace:*"',
                '"@destack/db": "workspace:*",\n        "@example/noisy": "workspace:*"',
            ),
        );

        // evaluate an ordinary imported title module and retain the complete initial output
        const file = join(directory, "src/database.ts");
        const source = await readFile(file, "utf8");
        await writeFile(
            file,
            'import { title } from "./title.ts";\n' +
                source.replace('text("title").notNull()', "text(title()).notNull()"),
        );
        const titleModule = join(directory, "src/title.ts");
        const original =
            '/** Return the column name. */\nexport function title(): string {\n    return "title";\n}\n';
        await writeFile(titleModule, original);
        const dependencies = await readDependencies(fileURLToPath(fixture));
        await using builder = await PackageBuilder.start(directory);
        await using first = await builder.build({ dependencies, ...resource.request });

        // reload the edited title module, whose dependency writes to standard output
        await writeFile(
            titleModule,
            'import { noisy } from "@example/noisy";\n\n/** Return the column name. */\nexport function title(): string {\n    return noisy("heading");\n}\n',
        );
        await using edited = await builder.build({ dependencies, ...resource.request });
        const baseline = present(
            (await first.reader.declarations()).find(
                (declaration) => declaration.kind === "resource",
            ),
            "the first build's resource",
        );
        const actual = present(
            (await edited.reader.declarations()).find(
                (declaration) => declaration.kind === "resource",
            ),
            "the edited build's resource",
        );
        const expected = structuredClone(baseline);
        const { description } = expected;
        if (!isTableDescription(description)) {
            throw new TypeError("the resource description has no tables");
        }
        for (const dialect of ["sqlite", "postgresql"]) {
            // rename the column in the table
            const [state] = present(description.tables[dialect], `the ${dialect} tables`);
            const columns = present(state, `the ${dialect} table`).table.columns;
            present(columns[1], "the title column").name = "heading";
        }
        expect(actual).toEqual(expected);

        // restore the title module and compare every distributed byte and the full manifest
        await writeFile(titleModule, original);
        await using restored = await builder.build({ dependencies, ...resource.request });
        expect(restored.manifest).toEqual(first.manifest);
        await expectFiles(restored, first);

        // await the same shutdown through concurrent callers and the enclosing using scope
        await Promise.all([builder[Symbol.asyncDispose](), builder[Symbol.asyncDispose]()]);
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("rebuild edited source, reject incompatible APIs, restore distributed output", async () => {
    const fixture = new URL("../../tests/fixture/library/", import.meta.url);
    const directory = await mkdtemp(join(tmpdir(), "destack-build-library-"));
    const source = join(directory, "source");
    const destination = join(directory, "build");

    try {
        // compile a real package
        await cp(new URL("source/", fixture), source, { recursive: true });
        await using builder = await PackageBuilder.start(source);
        await using build = await builder.build(request);

        // rebuild unchanged, edited, and restored source with the same compiler
        await using unchanged = await builder.build(request);
        expect(unchanged.manifest).toEqual(build.manifest);
        await expectFiles(unchanged, build);
        const notePath = join(source, "src/note.ts");
        const note = await readFile(notePath, "utf8");
        await writeFile(
            notePath,
            note
                .replace("complete: boolean", "complete: true")
                .replace("complete: false", "complete: true"),
        );
        await using edited = await builder.build(request);
        const editDirectory = join(directory, "edited");
        await edited.write(editDirectory);
        const editedModule = await importNotes(
            editDirectory,
            rootExport(edited.manifest, "library"),
        );
        expect(editedModule.createNote("Edited")).toEqual({ title: "Edited", complete: true });
        await writeFile(notePath, note);
        await using restoredSource = await builder.build(request);
        expect(restoredSource.manifest).toEqual(build.manifest);
        await expectFiles(restoredSource, build);

        // reject an API unavailable in the selected runtime, then accept restored source
        await writeFile(
            notePath,
            note + "\n/** The page title. */\nexport const page = document.title;\n",
        );
        await expect(
            builder.build({
                dependencies: {},
                outputs: {
                    library: { ...request.outputs.library, runtime: "workerd" },
                },
            }),
        ).rejects.toMatchObject({
            code: "BUILD_FAILED",
            message: `unsupported workerd API: document.title at src/note.ts:${note.length + 44}`,
        });

        // refuse the Function constructor on workerd, and accept reads of its prototype
        const constructed =
            note +
            "\n/** The constructor. */\nexport const create: FunctionConstructor = Function;\n";
        await writeFile(notePath, constructed);
        await expect(
            builder.build({
                dependencies: {},
                outputs: { library: { ...request.outputs.library, runtime: "workerd" } },
            }),
        ).rejects.toMatchObject({
            code: "BUILD_FAILED",
            message: `unsupported workerd API: Function at src/note.ts:${note.length + 68}`,
        });
        const prototypeSource =
            note +
            "\n/** The function prototype's names. */\nexport const names = Object.getOwnPropertyNames(Function.prototype);\n";
        await writeFile(notePath, prototypeSource);
        await using workerd = await builder.build({
            dependencies: {},
            outputs: { library: { ...request.outputs.library, runtime: "workerd" } },
        });
        expect(await readFile(join(workerd.directory, "src/note.ts"), "utf8")).toBe(
            prototypeSource,
        );

        // accept browser index signatures through the runtime's actual type declarations
        const browserSource =
            note +
            '\n/** The page theme. */\nexport const page = document.documentElement.dataset["theme"];\n';
        await writeFile(notePath, browserSource);
        await using browser = await builder.build({
            dependencies: {},
            outputs: {
                library: {
                    ...request.outputs.library,
                    runtime: "browser",
                },
            },
        });
        expect(await readFile(join(browser.directory, "src/note.ts"), "utf8")).toBe(browserSource);

        // remove source access before loading the distributed package
        await build.write(destination);
        await rm(source, { recursive: true });
        const restored = await PackageBuild.read(destination);
        expect(restored.manifest).toEqual(build.manifest);

        // preserve an existing distribution when the caller repeats a write
        await expect(build.write(destination)).rejects.toMatchObject({ code: "EEXIST" });
        expect((await PackageBuild.read(destination)).manifest).toEqual(build.manifest);

        // leave no published manifest when an output cannot be written
        const failed = join(directory, "failed");
        await rm(join(build.directory, rootExport(build.manifest, "library")));
        await expect(build.write(failed)).rejects.toMatchObject({ code: "ENOENT" });
        await expect(readFile(join(failed, "manifest.json"))).rejects.toMatchObject({
            code: "ENOENT",
        });
        const module = await importNotes(destination, rootExport(restored.manifest, "library"));
        expect(module.createNote("A note")).toEqual({ title: "A note", complete: false });

        // discard temporary compiler output while preserving the retained distribution
        await build[Symbol.asyncDispose]();
        await expect(readFile(join(build.directory, "manifest.json"))).rejects.toMatchObject({
            code: "ENOENT",
        });
        await restored[Symbol.asyncDispose]();
        expect((await PackageBuild.read(destination)).manifest).toEqual(restored.manifest);
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("reuse cached builds and outputs, and compile what their keys miss", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-build-cache-"));
    const source = join(directory, "source");
    try {
        // build a library into a store's cache
        await cp(new URL("../../tests/fixture/library/source/", import.meta.url), source, {
            recursive: true,
        });
        await using bucket = await LocalBucket.open(join(directory, "bucket"), "space-test");
        const store = new PackageStore(bucket);
        await using builder = await PackageBuilder.start(source);
        await using first = await builder.build({ ...request, store });
        expect(first.reused).toEqual([]);

        // restore the whole unchanged build from the cache, file for file
        await using unchanged = await builder.build({ ...request, store });
        expect(unchanged.reused).toEqual(["library"]);
        expect(unchanged.manifest).toEqual(first.manifest);
        await expectFiles(unchanged, first);

        // reuse the library output beside a module it does not import, as a build without the cache compiles it
        const draft = join(source, "src/draft.ts");
        await writeFile(
            draft,
            await formatSource(draft, '/** A draft title. */\nexport const draft = "draft";\n'),
        );
        await using added = await builder.build({ ...request, store });
        await using cold = await builder.build(request);
        expect(added.reused).toEqual(["library"]);
        expect(added.manifest).toEqual(cold.manifest);
        await expectFiles(added, cold);

        // compile the library again once a module it imports changes
        const note = join(source, "src/note.ts");
        const original = await readFile(note, "utf8");
        await writeFile(note, original.replace("complete: false", "complete: true"));
        await using edited = await builder.build({ ...request, store });
        expect(edited.reused).toEqual([]);
        expect(edited.manifest).not.toEqual(added.manifest);

        // restore the earlier build from the cache once the module is restored
        await writeFile(note, original);
        await using restored = await builder.build({ ...request, store });
        expect(restored.reused).toEqual(["library"]);
        expect(restored.manifest).toEqual(added.manifest);
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("evaluate package code without the host's environment, files or network", async () => {
    const fixture = new URL("../../tests/fixture/resource/source/", import.meta.url);
    const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-build-sandbox-")));
    const source = join(directory, "source");
    using server = Bun.serve({ port: 0, fetch: () => new Response("reached") });
    try {
        // copy the fixture beside its installed dependencies, and keep a host file outside it
        await cp(fixture, source, {
            recursive: true,
            filter: (path) => !path.endsWith("/node_modules") && !path.endsWith("\\node_modules"),
        });
        await mkdir(join(source, "node_modules"));
        await symlink(
            fileURLToPath(new URL("node_modules/@destack", fixture)),
            join(source, "node_modules/@destack"),
            "junction",
        );
        const secret = join(directory, "secret.txt");
        await writeFile(secret, "secret");

        // name a column by what package code observes of the host while the build evaluates it
        const probe = join(source, "src/probe.ts");
        await writeFile(
            probe,
            await formatSource(
                probe,
                `/** The host process, as Bun provides it. */
declare const process: Readonly<Record<string, Readonly<Record<string, string>> | undefined>>;

/** Bun's file reader. */
declare const Bun: { file(path: string): { text(): Promise<string> } };

/** The host's user, the host file's text or its refusal, and the host server's reply. */
export const observed = [
    process["env"]?.["USER"],
    await read(${JSON.stringify(secret)}),
    (await fetch(${JSON.stringify(server.url.href)})).status,
].join("_");

/** Read a file's text, or the code of the error refusing it. */
async function read(path: string): Promise<string> {
    try {
        return await Bun.file(path).text();
    } catch (error) {
        return error instanceof Error && "code" in error ? String(error.code) : String(error);
    }
}
`,
            ),
        );
        const database = join(source, "src/database.ts");
        await writeFile(
            database,
            'import { observed } from "./probe.ts";\n' +
                (await readFile(database, "utf8")).replace('text("title")', "text(observed)"),
        );

        // see no user, a refused file and the sandbox proxy's refusal
        await using builder = await PackageBuilder.start(source);
        const dependencies = await readDependencies(fileURLToPath(fixture));
        await using build = await builder.build({ dependencies, ...resource.request });
        const declared = present(
            (await build.reader.declarations()).find(
                (declaration) => declaration.kind === "resource",
            ),
            "the resource",
        );
        const { description } = declared;
        if (!isTableDescription(description)) {
            throw new TypeError("the resource description has no tables");
        }
        const [state] = present(description.tables["sqlite"], "the sqlite tables");
        const columns = present(state, "the note table").table.columns;
        expect(columns.map((column) => column.name)).toEqual([
            "id",
            `_${DENIED_READ[process.platform] ?? "unsupported"}_403`,
        ]);
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("reject invalid outputs and recover the retained compiler", async () => {
    // build once before any rejected request
    const source = fileURLToPath(new URL("../../tests/fixture/library/source/", import.meta.url));
    await using builder = await PackageBuilder.start(source);
    await using first = await builder.build(request);
    await expect(builder.build({ dependencies: {}, outputs: {} })).rejects.toMatchObject({
        code: "BUILD_FAILED",
        message: "a build requires at least one output",
    });
    await expect(
        builder.build({
            dependencies: {},
            outputs: { website: { kind: "web", app: "src/app.tsx", ssr: false } },
        }),
    ).rejects.toMatchObject({
        code: "BUILD_FAILED",
        message: "no dependency compiles outputs of kind web",
    });

    // build the same package again with the same compiler after rejected requests
    await using build = await builder.build(request);
    expect(build.manifest).toEqual(first.manifest);
});

test("refuse an output name a kind's expansion takes", async () => {
    // expand a web application beside a module output taking its browser output's name
    await using fixture = await Fixture.open("web");
    await using builder = await PackageBuilder.start(fixture.source);
    await expect(
        builder.build({
            dependencies: fixture.dependencies,
            outputs: {
                "website-browser": { kind: "module", runtime: "browser" },
                website: { kind: "web", app: "src/app.tsx", ssr: false },
            },
        }),
    ).rejects.toMatchObject({
        code: "BUILD_FAILED",
        message: "duplicate output name: website-browser",
    });
});

test("terminate compilation on cancellation and deadline, and build again with the same builder", async () => {
    const directory = fileURLToPath(
        new URL("../../tests/fixture/library/source/", import.meta.url),
    );
    const controller = new AbortController();
    await using builder = await PackageBuilder.start(directory);
    const pending = builder.build({ ...request, signal: controller.signal });
    controller.abort();
    await expect(pending).rejects.toMatchObject({
        code: "BUILD_FAILED",
        message: "build cancelled",
    });

    // stop a compiler past its deadline, and build again with the same builder
    await expect(builder.build({ ...request, timeout: 1 })).rejects.toMatchObject({
        code: "BUILD_FAILED",
        message: "build exceeded 1 ms",
    });
    await using rebuilt = await builder.build(request);
    expect(rebuilt.manifest.package.name).toBe("@destack/build-library-fixture");
    await expect(buildPackage({ ...request, directory, timeout: 1 })).rejects.toMatchObject({
        code: "BUILD_FAILED",
        message: "build exceeded 1 ms",
    });
});

test("plan the upgrade from what a package published", async () => {
    await using fixture = await Fixture.open("service");
    const options = {
        dependencies: fixture.dependencies,
        outputs: { bun: service.request.outputs.bun },
    };
    const file = join(fixture.source, "src/server.ts");
    const manifest = join(fixture.source, "package.json");
    const original = await readFile(file, "utf8");
    await using builder = await PackageBuilder.start(fixture.source);

    // publish the first release
    await using first = await builder.build(options);
    const published = await first.reader.declarations();
    const history = {
        release: "2026.9.0",
        declarations: published,
        vocabulary: Vocabulary.advance({}, published, "2026.9.0"),
    };

    // rename the audit action and add a procedure in the next release
    const release = async (version: string, source: string) => {
        const definition = await readFile(manifest, "utf8");
        await writeFile(
            manifest,
            definition.replace(/"version": "[^"]+"/u, `"version": "${version}"`),
        );
        await writeFile(file, await formatSource(file, source));
    };
    const renamed = original
        .replace('name: "note.publish"', 'name: "note.release"')
        .replace(
            "        .output(schema.object({ path: schema.string() })),\n};",
            '        .output(schema.object({ path: schema.string() })),\n    count: defineProcedure({ authentication: "public", permission: null, audit: false })\n        .route({ method: "GET", path: "/notes/count" })\n        .output(schema.number()),\n};',
        )
        .replace(
            'list: implementation.list.handler(() => ({ path: "/notes" })),',
            'list: implementation.list.handler(() => ({ path: "/notes" })),\n            count: implementation.count.handler(() => 0),',
        );
    await release("2026.10.0", renamed);
    await using second = await builder.build({ ...options, history });
    const upgrade: unknown = JSON.parse(
        await readFile(
            join(second.directory, present(second.manifest.upgrade, "the upgrade").file.path),
            "utf8",
        ),
    );
    expect(upgrade).toEqual({
        from: "2026.9.0",
        steps: [
            {
                action: "create",
                target: "audit-action/note.release",
                risk: "safe",
                detail: "add audit-action",
            },
            {
                action: "create",
                target: "service/notes/procedure/count",
                risk: "safe",
                detail: "add procedure count",
            },
            {
                action: "delete",
                target: "audit-action/note.publish",
                risk: "backward-incompatible",
                detail: "remove: data stored under it no longer applies",
            },
        ],
    });
});

test("bundle an empty module for a host module a dependency's browser field replaces, and refuse an unreplaced one", async () => {
    const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-build-replaced-")));
    try {
        // write an app using a sibling parser, whose file reader requires the file system lazily
        const parser = {
            name: "@example/parser",
            version: "2026.9.0",
            main: "./index.js",
            types: "./index.d.ts",
            browser: { "fs/promises": false },
        };
        const files: Record<string, string> = {
            "bun.lock": JSON.stringify({ lockfileVersion: 1, workspaces: {}, packages: {} }),
            "app/destack.json": JSON.stringify({
                $schema: "https://destack.app/schemas/2026.9.0/destack.json",
                id: "package-01996ab0-0000-7000-8000-00000000000c",
                language: "typescript",
                runtimes: ["workerd"],
            }),
            "app/package.json": JSON.stringify({
                name: "@example/app",
                version: "2026.9.0",
                type: "module",
                exports: { ".": "./src/greeting.ts" },
                dependencies: { "@example/parser": "workspace:*" },
            }),
            "app/src/greeting.ts":
                'import { parse } from "@example/parser";\n\n/** The parsed greeting. */\nexport const greeting: string = parse("hello");\n',
            "parser/destack.json": JSON.stringify({
                $schema: "https://destack.app/schemas/2026.9.0/destack.json",
                id: "package-01996ab0-0000-7000-8000-00000000000d",
                language: "typescript",
                runtimes: ["workerd"],
            }),
            "parser/package.json": JSON.stringify(parser),
            "parser/index.js":
                'exports.parse = (text) => text.toUpperCase();\nexports.parseFile = async (path) => {\n    const { readFile } = require("fs/promises");\n    return exports.parse(await readFile(path, "utf8"));\n};\n',
            "parser/index.d.ts":
                "export declare function parse(text: string): string;\nexport declare function parseFile(path: string): Promise<string>;\n",
        };
        for (const [path, text] of Object.entries(files)) {
            await mkdir(join(directory, path, ".."), { recursive: true });
            await writeFile(join(directory, path), text);
        }
        await mkdir(join(directory, "app/node_modules/@example"), { recursive: true });
        await symlink(
            join(directory, "parser"),
            join(directory, "app/node_modules/@example/parser"),
        );

        // run the parser built for workerd, which lacks the file system
        await using builder = await PackageBuilder.start(join(directory, "app"));
        const outputs = { server: { kind: "module", runtime: "workerd", bundle: true } } as const;
        await using built = await builder.build({ dependencies: {}, outputs });
        await built.write(join(directory, "build"));
        const module = await importBuilt(
            join(directory, "build"),
            rootExport(built.manifest, "server"),
        );
        expect({ ...module }).toEqual({ greeting: "HELLO" });

        // refuse the file system once the parser no longer replaces it
        const { browser: _browser, ...unreplaced } = parser;
        await writeFile(join(directory, "parser/package.json"), JSON.stringify(unreplaced));
        await expect(builder.build({ dependencies: {}, outputs })).rejects.toMatchObject({
            code: "BUILD_FAILED",
            message: "host module is unavailable on workerd: fs/promises",
        });
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("compile a package with the build extension it declares", async () => {
    const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-build-extension-")));
    try {
        // write a package whose module reads a constant only its own extension writes in
        const files: Record<string, string> = {
            "bun.lock": JSON.stringify({ lockfileVersion: 1, workspaces: {}, packages: {} }),
            "destack.json": JSON.stringify({
                $schema: "https://destack.app/schemas/2026.9.0/destack.json",
                id: "package-01996ab0-0000-7000-8000-00000000000e",
                language: "typescript",
                runtimes: ["bun"],
                build: "./build#greetingExtension",
            }),
            "package.json": JSON.stringify({
                name: "@example/greeting",
                version: "2026.9.0",
                type: "module",
                exports: { ".": "./src/greeting.ts", "./build": "./src/build.ts" },
            }),
            "src/greeting.ts": [
                "/** The greeting the build extension writes in. */",
                "declare const GREETING: string;",
                "",
                "/** The greeting. */",
                "export const greeting: string = GREETING;",
                "",
            ].join("\n"),
            "src/build.ts": [
                "/** Write the greeting into the package's modules. */",
                "export const greetingExtension = {",
                "    compile: () => [",
                "        {",
                '            name: "greeting",',
                "            transform: (code: string) => ({",
                "                code: code.replaceAll(/\\bGREETING\\b/gu, '\"hello\"'),",
                "                map: null,",
                "            }),",
                "        },",
                "    ],",
                "};",
                "",
            ].join("\n"),
        };
        for (const [path, text] of Object.entries(files)) {
            await mkdir(join(directory, path, ".."), { recursive: true });
            await writeFile(join(directory, path), text);
        }

        // build the package and read the greeting its extension wrote in
        await using builder = await PackageBuilder.start(directory);
        const outputs = { library: { kind: "module", runtime: "bun", bundle: true } } as const;
        await using built = await builder.build({ dependencies: {}, outputs });
        await built.write(join(directory, "build"));
        const module = await importBuilt(
            join(directory, "build"),
            rootExport(built.manifest, "library"),
        );
        expect({ ...module }).toEqual({ greeting: "hello" });
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("bundle a dependency whose optional peer is absent, failing only once it is imported", async () => {
    const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-build-peer-")));
    try {
        // write an app using a sibling loader, whose optional peer nobody installs
        const files: Record<string, string> = {
            "bun.lock": JSON.stringify({ lockfileVersion: 1, workspaces: {}, packages: {} }),
            "app/destack.json": JSON.stringify({
                $schema: "https://destack.app/schemas/2026.9.0/destack.json",
                id: "package-01996ab0-0000-7000-8000-00000000000f",
                language: "typescript",
                runtimes: ["bun"],
            }),
            "app/package.json": JSON.stringify({
                name: "@example/app",
                version: "2026.9.0",
                type: "module",
                exports: { ".": "./src/app.ts" },
                dependencies: { "@example/loader": "workspace:*" },
            }),
            "app/src/app.ts": 'export { greet, load } from "@example/loader";\n',
            "loader/destack.json": JSON.stringify({
                $schema: "https://destack.app/schemas/2026.9.0/destack.json",
                id: "package-01996ab0-0000-7000-8000-000000000010",
                language: "typescript",
                runtimes: ["bun"],
            }),
            "loader/package.json": JSON.stringify({
                name: "@example/loader",
                version: "2026.9.0",
                type: "module",
                main: "./index.js",
                types: "./index.d.ts",
                peerDependencies: { "@example/plugin": "2026.9.0" },
                peerDependenciesMeta: { "@example/plugin": { optional: true } },
            }),
            "loader/index.js":
                'export const greet = () => "hello";\nexport const load = () => import("@example/plugin");\n',
            "loader/index.d.ts":
                "export declare function greet(): string;\nexport declare function load(): Promise<unknown>;\n",
        };
        for (const [path, text] of Object.entries(files)) {
            await mkdir(join(directory, path, ".."), { recursive: true });
            await writeFile(join(directory, path), text);
        }
        await mkdir(join(directory, "app/node_modules/@example"), { recursive: true });
        await symlink(
            join(directory, "loader"),
            join(directory, "app/node_modules/@example/loader"),
        );

        // greet through the bundled loader, and fail to load the absent peer
        await using builder = await PackageBuilder.start(join(directory, "app"));
        const outputs = { library: { kind: "module", runtime: "bun", bundle: true } } as const;
        await using built = await builder.build({ dependencies: {}, outputs });
        await built.write(join(directory, "build"));
        const module = await importBuilt(
            join(directory, "build"),
            rootExport(built.manifest, "library"),
        );
        if (!isLoaderModule(module)) {
            throw new TypeError("the app exports no greet and load functions");
        }
        expect(module.greet()).toBe("hello");
        await expect(module.load()).rejects.toMatchObject({
            message:
                'Could not resolve "@example/plugin" imported by "@example/loader". Is it installed?',
        });
    } finally {
        await rm(directory, { recursive: true });
    }
});

/** Read the file an output exports at its root, failing when the manifest lacks it. */
function rootExport(manifest: PackageManifest, output: string): string {
    const path = manifest.outputs[output]?.exports["."];
    if (path === undefined) {
        throw new Error(`the manifest has no root export of ${output}`);
    }

    return path;
}

/** Report whether a built module exports the loader's greet and load functions. */
function isLoaderModule(module: object): module is { greet(): string; load(): Promise<unknown> } {
    return (
        "greet" in module &&
        typeof module.greet === "function" &&
        "load" in module &&
        typeof module.load === "function"
    );
}

/** Report whether a resource description lists tables by dialect, keeping the parsed objects. */
function isTableDescription(value: unknown): value is TableDescription {
    return TableDescription.safeParse(value).success;
}

/** Import a written build's library module, requiring its createNote function. */
async function importNotes(destination: string, path: string): Promise<NoteModule> {
    const module = await importBuilt(destination, path);
    if (!isNoteModule(module)) {
        throw new TypeError(`${path} exports no createNote function`);
    }

    return module;
}
