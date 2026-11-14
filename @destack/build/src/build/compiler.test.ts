import { yieldToEventLoop } from "@destack/environment";
import { testSimulation } from "../test/world.ts";
import { expect, refusal, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { PackageBuild, Compiler } from "../index.ts";
import type { PackageManifest } from "@destack/package/manifest";
import { request } from "../../tests/fixture/library/request.ts";
import { expectFiles, importBuilt, isNoteModule, type NoteModule } from "../../tests/fixture.ts";
import { present } from "@destack/schema";
import { LocalBuildStore } from "../store/local.ts";
import { BuildStore } from "../store/store.ts";
import { Toolchain } from "@destack/check/toolchain";

/** How long a cancelled build runs before it aborts: well inside the library's build. */
const CANCEL_MILLISECONDS = 100;

test("rebuild edited source, reject incompatible APIs, restore distributed output", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const fixture = new URL("../../tests/fixture/library/", import.meta.url);
    const directory = await files.makeTemporaryDirectory("build-library-");
    const source = join(directory, "source");
    const destination = join(directory, "build");
    const library = { ...request, directory: source };

    try {
        // compile a real package
        await files.copy(fileURLToPath(new URL("source/", fixture)), source);
        await using store = await LocalBuildStore.temporary(environment);
        await using builder = await Compiler.start(source, {
            store,
            toolchain: async () => Toolchain.workspace(),
        });
        const build = await builder.build(library);

        // rebuild unchanged, edited, and restored source with the same compiler
        const unchanged = await builder.build(library);
        expect(unchanged.manifest).toEqual(build.manifest);
        await expectFiles(unchanged, build);
        const notePath = join(source, "src/note.ts");
        const note = await files.readTextFile(notePath);
        await files.writeTextFile(
            notePath,
            note
                .replace("complete: boolean", "complete: true")
                .replace("complete: false", "complete: true"),
        );
        const edited = await builder.build(library);
        const editDirectory = join(directory, "edited");
        await edited.writeDirectory(editDirectory, files);
        const editedModule = await importNotes(
            editDirectory,
            rootExport(edited.manifest, "library"),
        );
        expect(editedModule.createNote("Edited")).toEqual({ title: "Edited", complete: true });
        await files.writeTextFile(notePath, note);
        const restoredSource = await builder.build(library);
        expect(restoredSource.manifest).toEqual(build.manifest);
        await expectFiles(restoredSource, build);

        // refuse a browser API in the checked workerd output, though the package also ships the module to the browser
        const workerdRequest = {
            directory: source,
            dependencies: {},
            links: {},
            outputs: { library: { ...request.outputs.library, runtime: "workerd" } },
        } as const;
        await files.writeTextFile(
            notePath,
            note + "\n/** The page title. */\nexport const page = document.title;\n",
        );
        expect(await refusal(builder.check(workerdRequest))).toEqual([
            "BAD_REQUEST",
            "RUNTIME_API_INVALID",
            `API 'document.title' at 'src/note.ts:${note.length + 44}' is invalid: it is not supported on 'workerd'`,
        ]);

        // accept the browser API inside a function, which only the browser calls
        const deferredSource =
            note + "\n/** Read the page title. */\nexport const page = () => document.title;\n";
        await files.writeTextFile(notePath, deferredSource);
        const deferred = await builder.check(workerdRequest);
        expect(new TextDecoder().decode(await deferred.load("src/note.ts"))).toBe(deferredSource);

        // refuse the Function constructor on workerd, and accept reads of its prototype
        const constructed =
            note +
            "\n/** The constructor. */\nexport const create: FunctionConstructor = Function;\n";
        await files.writeTextFile(notePath, constructed);
        expect(
            await refusal(
                builder.check({
                    directory: source,
                    dependencies: {},
                    links: {},
                    outputs: { library: { ...request.outputs.library, runtime: "workerd" } },
                }),
            ),
        ).toEqual([
            "BAD_REQUEST",
            "RUNTIME_API_INVALID",
            `API 'Function' at 'src/note.ts:${note.length + 68}' is invalid: it is not supported on 'workerd'`,
        ]);
        const prototypeSource =
            note +
            "\n/** The function prototype's names. */\nexport const names = Object.getOwnPropertyNames(Function.prototype);\n";
        await files.writeTextFile(notePath, prototypeSource);
        const workerd = await builder.check({
            directory: source,
            dependencies: {},
            links: {},
            outputs: { library: { ...request.outputs.library, runtime: "workerd" } },
        });
        expect(new TextDecoder().decode(await workerd.load("src/note.ts"))).toBe(prototypeSource);

        // accept a global read by a well-known symbol, and refuse one read by a computed key
        const symbolSource =
            note + "\n/** The tag of Math. */\nexport const tag = Math[Symbol.toStringTag];\n";
        await files.writeTextFile(notePath, symbolSource);
        const symbolKeyed = await builder.check(workerdRequest);
        expect(new TextDecoder().decode(await symbolKeyed.load("src/note.ts"))).toBe(symbolSource);
        await files.writeTextFile(
            notePath,
            note +
                '\n/** A Math member. */\nexport const member = (key: string) => Math[key as "PI"];\n',
        );
        expect(await refusal(builder.check(workerdRequest))).toEqual([
            "BAD_REQUEST",
            "RUNTIME_API_INVALID",
            `global access to 'Math' at 'src/note.ts:${note.length + 62}' is invalid: it is dynamic`,
        ]);

        // accept browser index signatures through the runtime's actual type declarations
        const browserSource =
            note +
            '\n/** Read the page theme. */\nexport const page = () => document.documentElement.dataset["theme"];\n';
        await files.writeTextFile(notePath, browserSource);
        const browser = await builder.check({
            directory: source,
            dependencies: {},
            links: {},
            outputs: {
                library: {
                    ...request.outputs.library,
                    runtime: "browser",
                },
            },
        });
        expect(new TextDecoder().decode(await browser.load("src/note.ts"))).toBe(browserSource);

        // remove source access before loading the distributed package
        await build.writeDirectory(destination, files);
        await files.remove(source, { recursive: true });
        const restored = await PackageBuild.read(destination, environment);
        expect(restored.manifest).toEqual(build.manifest);

        // preserve an existing distribution when the caller repeats a write
        expect(await refusal(build.writeDirectory(destination, files))).toEqual([
            "CONFLICT",
            "FILE_EXISTS",
            `cannot create ${destination}: EEXIST`,
        ]);
        expect((await PackageBuild.read(destination, environment)).manifest).toEqual(
            build.manifest,
        );

        // refuse writing a file of a restored copy whose bytes of the same size no longer match its digest
        await using copied = await PackageBuild.restore(
            store,
            await BuildStore.digest(build.manifest, environment.crypto),
            join(directory, "copied"),
        );
        const changed = present(
            (await copied.reader.distributed()).find(
                (file) => file.path !== rootExport(build.manifest, "library"),
            ),
            "a distributed file besides the library",
        );
        const original = await files.readFile(join(copied.directory, changed.path));
        const flipped = Uint8Array.from(original, (byte, index) => (index === 0 ? byte ^ 1 : byte));
        await files.writeFile(join(copied.directory, changed.path), flipped);
        const corrupted = join(directory, "corrupted");
        expect(await refusal(copied.reader.writeDirectory(corrupted, files))).toEqual([
            "BAD_REQUEST",
            "FILE_INVALID",
            `file '${changed.path}' is invalid: its digest does not match`,
        ]);
        await files.writeFile(join(copied.directory, changed.path), original);

        // leave no published manifest when an output cannot be written
        const failed = join(directory, "failed");
        await files.remove(join(copied.directory, rootExport(build.manifest, "library")));
        const removed = join(copied.directory, rootExport(build.manifest, "library"));
        expect(await refusal(copied.reader.writeDirectory(failed, files))).toEqual([
            "NOT_FOUND",
            "FILE_NOT_FOUND",
            `cannot open ${removed}: ENOENT`,
        ]);
        expect(await refusal(files.readFile(join(failed, "manifest.json")))).toEqual([
            "NOT_FOUND",
            "FILE_NOT_FOUND",
            `cannot read ${join(failed, "manifest.json")}: ENOENT`,
        ]);

        const module = await importNotes(destination, rootExport(restored.manifest, "library"));
        expect(module.createNote("A note")).toEqual({ title: "A note", complete: false });

        // preserve the retained distribution on disposal
        await restored[Symbol.asyncDispose]();
        expect((await PackageBuild.read(destination, environment)).manifest).toEqual(
            restored.manifest,
        );
    } finally {
        await files.remove(directory, { recursive: true });
    }
});

test("reject invalid outputs and recover the retained compiler", async () => {
    // build once before any rejected request
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const source = fileURLToPath(new URL("../../tests/fixture/library/source/", import.meta.url));
    const library = { ...request, directory: source };
    await using store = await LocalBuildStore.temporary(environment);
    await using builder = await Compiler.start(source, {
        store,
        toolchain: async () => Toolchain.workspace(),
    });
    const first = await builder.build(library);
    expect(
        await refusal(
            builder.build({ directory: source, dependencies: {}, links: {}, outputs: {} }),
        ),
    ).toEqual([
        "BAD_REQUEST",
        "OUTPUT_INVALID",
        "build output set is invalid: a build requires at least one output",
    ]);
    expect(
        await refusal(
            builder.build({
                directory: source,
                dependencies: {},
                links: {},
                outputs: { Library: { kind: "module", runtime: "bun" } },
            }),
        ),
    ).toEqual([
        "BAD_REQUEST",
        "OUTPUT_INVALID",
        "output name 'Library' is invalid: it must be a valid name",
    ]);

    // build the same package again with the same compiler after rejected requests
    const build = await builder.build(library);
    expect(build.manifest).toEqual(first.manifest);
});

test("terminate compilation on cancellation and deadline, and build again with the same builder", async () => {
    // run on the host, as a compiler's deadline bounds the host's time a build takes
    const environment = testHost();
    const directory = fileURLToPath(
        new URL("../../tests/fixture/library/source/", import.meta.url),
    );
    const library = { ...request, directory };
    const controller = new AbortController();
    await using store = await LocalBuildStore.temporary(environment);
    await using builder = await Compiler.start(directory, {
        store,
        toolchain: async () => Toolchain.workspace(),
    });

    // cancel a build once the compiler has it, as the library's build takes over a second
    const pending = builder.build({ ...library, signal: controller.signal });
    void environment.clock.sleep(CANCEL_MILLISECONDS).then(() => controller.abort());
    expect(await refusal(pending)).toEqual([
        "CLIENT_CLOSED_REQUEST",
        "BUILD_CANCELLED",
        "build was cancelled",
    ]);

    // stop a compiler past its deadline, and build again with the same builder
    expect(await refusal(builder.build({ ...library, timeout: 1 }))).toEqual([
        "BAD_GATEWAY",
        "COMPILER_UNREACHABLE",
        "could not finish the build within 1 ms",
    ]);
    const rebuilt = await builder.build(library);
    expect(rebuilt.manifest.package.name).toBe("@destack/build-library-fixture");
});

test("close a compiler once it exits, however long its shutdown takes", async () => {
    // start a compiler that exits only once the test releases it, after the end of its input
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.realPath(await files.makeTemporaryDirectory("destack-shutdown-"));
    const release = join(directory, "release");
    const script = join(directory, "compiler.ts");
    await files.writeTextFile(
        script,
        `for await (const _chunk of process.stdin) {}
while (!(await Bun.file(${JSON.stringify(release)}).exists())) {
    await Bun.sleep(10);
}
`,
    );
    const source = fileURLToPath(new URL("../../tests/fixture/library/source/", import.meta.url));
    await using store = await LocalBuildStore.temporary(environment);
    const builder = await Compiler.start(source, {
        command: {
            executable: environment.process.executable,
            arguments: ["run", "--no-env-file", script],
            read: [directory],
            variables: {},
        },
        store,
        toolchain: async () => Toolchain.workspace(),
    });

    // stay open an hour past the end of its input, then close once it exits
    try {
        let isClosed = false;
        const closing = builder[Symbol.asyncDispose]().then(() => {
            isClosed = true;
        });
        await simulation.jump(simulation.clock.now() + 3_600_000);
        await yieldToEventLoop();
        const isClosedEarly = isClosed;
        await files.writeTextFile(release, "");
        await closing;
        expect([isClosedEarly, isClosed]).toEqual([false, true]);
    } finally {
        await files.remove(directory, { recursive: true });
    }
});

test("compile a package with the build extension it declares", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.realPath(
        await files.makeTemporaryDirectory("destack-build-extension-"),
    );
    try {
        // write a package whose module reads a constant only its extension writes in
        const contents: Record<string, string> = {
            "bun.lock": JSON.stringify({ lockfileVersion: 1, workspaces: {}, packages: {} }),
            "destack.json": JSON.stringify({
                $schema: "https://destack.app/schemas/2026.9.0/destack.json",
                id: "package-01996ab0-0000-7000-8000-00000000000e",
                role: "library",
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
                "    transform: () => [",
                "        {",
                '            name: "greeting",',
                "            transform: (code: string) => ({",
                "                code: code.replaceAll('= GREETING', '= \"hello\"'),",
                "                map: null,",
                "            }),",
                "        },",
                "    ],",
                "};",
                "",
            ].join("\n"),
        };
        for (const [path, text] of Object.entries(contents)) {
            await files.mkdir(join(directory, path, ".."), { recursive: true });
            await files.writeTextFile(join(directory, path), text);
        }

        // build the package and read the greeting its extension wrote in
        await using store = await LocalBuildStore.temporary(environment);
        await using builder = await Compiler.start(directory, {
            store,
            toolchain: async () => Toolchain.workspace(),
        });
        const outputs = { library: { kind: "module", runtime: "bun" } } as const;
        const built = await builder.build({
            directory: directory,
            dependencies: {},
            links: {},
            outputs,
        });
        await built.writeDirectory(join(directory, "build"), files);
        const module = await importBuilt(
            join(directory, "build"),
            rootExport(built.manifest, "library"),
        );
        expect({ ...module }).toEqual({ greeting: "hello" });
    } finally {
        await files.remove(directory, { recursive: true });
    }
});

test("bundle a dependency's file imported with a query through the extension that loads it", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.realPath(
        await files.makeTemporaryDirectory("destack-build-query-"),
    );
    try {
        // write a package whose module imports a dependency's text file through its extension's query
        const contents: Record<string, string> = {
            "bun.lock": JSON.stringify({ lockfileVersion: 1, workspaces: {}, packages: {} }),
            "destack.json": JSON.stringify({
                $schema: "https://destack.app/schemas/2026.9.0/destack.json",
                id: "package-01996ab0-0000-7000-8000-00000000000f",
                role: "library",
                language: "typescript",
                runtimes: ["bun"],
                build: "./build#shoutExtension",
            }),
            "package.json": JSON.stringify({
                name: "@example/shout",
                version: "2026.9.0",
                type: "module",
                dependencies: { "@example/words": "1.0.0" },
                exports: { ".": "./src/shout.ts", "./build": "./src/build.ts" },
            }),
            "node_modules/@example/words/package.json": JSON.stringify({
                name: "@example/words",
                version: "1.0.0",
            }),
            "node_modules/@example/words/hello.txt": "hello",
            "src/shout.ts": [
                "// @ts-expect-error: the extension loads the query",
                'import hello from "@example/words/hello.txt?shout";',
                "",
                "/** The dependency's word, shouted. */",
                "export const shout: string = hello;",
                "",
            ].join("\n"),
            "src/build.ts": [
                'import { readFileSync } from "node:fs";',
                "",
                "/** Load each `?shout` import as its file's text in capitals. */",
                "export const shoutExtension = {",
                "    transform: () => [",
                "        {",
                '            name: "shout",',
                '            enforce: "pre",',
                "            load: (id: string) =>",
                '                id.endsWith("?shout")',
                "                    ? `export default ${JSON.stringify(",
                '                          readFileSync(id.slice(0, -"?shout".length), "utf8").toUpperCase(),',
                "                      )};`",
                "                    : null,",
                "        },",
                "    ],",
                "};",
                "",
            ].join("\n"),
        };
        for (const [path, text] of Object.entries(contents)) {
            await files.mkdir(join(directory, path, ".."), { recursive: true });
            await files.writeTextFile(join(directory, path), text);
        }

        // build the package and read the word its extension loaded
        await using store = await LocalBuildStore.temporary(environment);
        await using builder = await Compiler.start(directory, {
            store,
            toolchain: async () => Toolchain.workspace(),
        });
        const outputs = { library: { kind: "module", runtime: "bun" } } as const;
        const built = await builder.build({ directory, dependencies: {}, links: {}, outputs });
        await built.writeDirectory(join(directory, "build"), files);
        const module = await importBuilt(
            join(directory, "build"),
            rootExport(built.manifest, "library"),
        );
        expect({ ...module }).toEqual({ shout: "HELLO" });
    } finally {
        await files.remove(directory, { recursive: true });
    }
});

test("check every output's entries with the package's ambient declarations", async () => {
    // write a package for Bun and browsers whose entry uses a type its ambient declarations declare
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.realPath(
        await files.makeTemporaryDirectory("destack-build-ambient-"),
    );
    const contents: Record<string, string> = {
        "bun.lock": JSON.stringify({ lockfileVersion: 1, workspaces: {}, packages: {} }),
        "destack.json": JSON.stringify({
            $schema: "https://destack.app/schemas/2026.9.0/destack.json",
            id: "package-01996ab0-0000-7000-8000-000000000010",
            role: "library",
            language: "typescript",
            runtimes: ["bun", "browser"],
        }),
        "package.json": JSON.stringify({
            name: "@example/greeting",
            version: "2026.9.0",
            type: "module",
            exports: { ".": "./src/index.ts" },
        }),
        "tsconfig.json": JSON.stringify({
            compilerOptions: { strict: true, module: "Preserve", moduleResolution: "Bundler" },
            include: ["src"],
        }),
        "src/greeting.d.ts":
            "/** A greeting a page shows. */\ninterface Greeting {\n    readonly text: string;\n}\n",
        "src/index.ts":
            "/** Read a greeting's text. */\nexport const read = (greeting: Greeting): string => greeting.text;\n",
    };
    for (const [path, text] of Object.entries(contents)) {
        await files.mkdir(join(directory, path, ".."), { recursive: true });
        await files.writeTextFile(join(directory, path), text);
    }

    // build both outputs, the browser's program rooted at its entries
    await using store = await LocalBuildStore.temporary(environment);
    await using builder = await Compiler.start(directory, {
        store,
        toolchain: async () => Toolchain.workspace(),
    });
    const built = await builder.build({
        directory,
        dependencies: {},
        links: {},
        outputs: {
            server: { kind: "module", runtime: "bun" },
            browser: { kind: "module", runtime: "browser" },
        },
    });
    expect(Object.keys(built.manifest.outputs).toSorted()).toEqual(["browser", "server"]);
});

test("compile each command the package installs into its Bun output, keeping its hashbang", async () => {
    // write a package installing a command beside its root export
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.realPath(
        await files.makeTemporaryDirectory("destack-build-command-"),
    );
    const contents: Record<string, string> = {
        "bun.lock": JSON.stringify({ lockfileVersion: 1, workspaces: {}, packages: {} }),
        "destack.json": JSON.stringify({
            $schema: "https://destack.app/schemas/2026.9.0/destack.json",
            id: "package-01996ab0-0000-7000-8000-000000000011",
            role: "library",
            language: "typescript",
            runtimes: ["bun"],
        }),
        "package.json": JSON.stringify({
            name: "@example/greet",
            version: "2026.9.0",
            type: "module",
            bin: { greet: "./src/main.ts" },
            exports: { ".": "./src/index.ts" },
        }),
        "tsconfig.json": JSON.stringify({
            compilerOptions: { strict: true, module: "Preserve", moduleResolution: "Bundler" },
            include: ["src"],
        }),
        "src/index.ts": '/** The greeting. */\nexport const greeting = "hello";\n',
        "src/main.ts":
            '#!/usr/bin/env bun\nimport { greeting } from "./index.ts";\n\nconsole.log(greeting);\n',
    };
    for (const [path, text] of Object.entries(contents)) {
        await files.mkdir(join(directory, path, ".."), { recursive: true });
        await files.writeTextFile(join(directory, path), text);
    }

    // build the Bun output and read the command's emitted module
    await using store = await LocalBuildStore.temporary(environment);
    await using builder = await Compiler.start(directory, {
        store,
        toolchain: async () => Toolchain.workspace(),
    });
    const built = await builder.build({
        directory,
        dependencies: {},
        links: {},
        outputs: { bun: { kind: "module", runtime: "bun" } },
    });
    const bins = built.manifest.outputs["bun"]?.bins;
    expect(bins).toEqual({ greet: { path: "output/bun/src/main.js", imports: [] } });
    await built.writeDirectory(join(directory, "build"), files);
    const emitted = await files.readTextFile(join(directory, "build/output/bun/src/main.js"));
    expect(emitted.split("\n")[0]).toBe("#!/usr/bin/env bun");
});

test("keep the declaration file a package exports for another runtime in a Bun build", async () => {
    // write a Bun package exporting ambient declarations for browsers
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.realPath(
        await files.makeTemporaryDirectory("destack-build-declaration-"),
    );
    const contents: Record<string, string> = {
        "bun.lock": JSON.stringify({ lockfileVersion: 1, workspaces: {}, packages: {} }),
        "destack.json": JSON.stringify({
            $schema: "https://destack.app/schemas/2026.9.0/destack.json",
            id: "package-01996ab0-0000-7000-8000-000000000012",
            role: "library",
            language: "typescript",
            runtimes: ["bun"],
            exports: { "./browser": { runtimes: ["browser"] } },
        }),
        "package.json": JSON.stringify({
            name: "@example/ambient",
            version: "2026.9.0",
            type: "module",
            exports: { ".": "./src/index.ts", "./browser": "./src/browser.d.ts" },
        }),
        "tsconfig.json": JSON.stringify({
            compilerOptions: { strict: true, module: "Preserve", moduleResolution: "Bundler" },
            include: ["src"],
        }),
        "src/browser.d.ts": "/** A greeting browsers show. */\ndeclare const GREETING: string;\n",
        "src/index.ts": '/** The greeting. */\nexport const greeting = "hello";\n',
    };
    for (const [path, text] of Object.entries(contents)) {
        await files.mkdir(join(directory, path, ".."), { recursive: true });
        await files.writeTextFile(join(directory, path), text);
    }

    // build the Bun output and read the declaration file from the written build
    await using store = await LocalBuildStore.temporary(environment);
    await using builder = await Compiler.start(directory, {
        store,
        toolchain: async () => Toolchain.workspace(),
    });
    const built = await builder.build({
        directory,
        dependencies: {},
        links: {},
        outputs: { bun: { kind: "module", runtime: "bun" } },
    });
    await built.writeDirectory(join(directory, "build"), files);
    expect(await files.readTextFile(join(directory, "build/src/browser.d.ts"))).toBe(
        contents["src/browser.d.ts"],
    );
});

/** Read the file an output exports at its root, failing when the manifest lacks it. */
function rootExport(manifest: PackageManifest, output: string): string {
    const path = manifest.outputs[output]?.exports["."]?.path;
    if (path === undefined) {
        throw new Error(`the manifest has no root export of '${output}'`);
    }

    return path;
}

/** Import a written build's library module, requiring its createNote function. */
async function importNotes(destination: string, path: string): Promise<NoteModule> {
    const module = await importBuilt(destination, path);
    if (!isNoteModule(module)) {
        throw new TypeError(`'${path}' exports no createNote function`);
    }

    return module;
}

test("bundle a module a page starts as a worker, and refuse one it references as an asset", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.makeTemporaryDirectory("build-worker-");
    const source = join(directory, "source");
    const browser = {
        directory: source,
        dependencies: {},
        links: {},
        outputs: { library: { ...request.outputs.library, runtime: "browser" } },
    } as const;

    try {
        // add an echo worker to the library
        await files.copy(
            fileURLToPath(new URL("../../tests/fixture/library/source/", import.meta.url)),
            source,
        );
        await files.writeTextFile(
            join(source, "src/echo.worker.ts"),
            'self.addEventListener("message", (event) => self.postMessage(event.data));\n',
        );
        const notePath = join(source, "src/note.ts");
        const note = await files.readTextFile(notePath);
        await using store = await LocalBuildStore.temporary(environment);
        await using builder = await Compiler.start(source, {
            store,
            toolchain: async () => Toolchain.workspace(),
        });

        // start it as a worker, then reference it as an asset
        await files.writeTextFile(
            notePath,
            `${note}\n/** Start the echo worker. */\nexport const echo = () =>\n    new Worker(new URL("./echo.worker.ts", import.meta.url), { type: "module" });\n`,
        );
        const started = await builder.build(browser);
        const workers = (await started.files()).filter((file) =>
            /echo\.worker[^/]*\.js$/u.test(file.path),
        );
        await files.writeTextFile(
            notePath,
            `${note}\n/** The echo worker's address. */\nexport const echo = new URL("./echo.worker.ts", import.meta.url).href;\n`,
        );
        const referenced = await refusal(builder.build(browser));

        // expect one bundled JavaScript worker, and the asset reference refused
        expect({
            workers: workers.map((file) => [file.path.endsWith(".js"), file.mediaType]),
            referenced: referenced === "done" ? referenced : referenced.slice(0, 2),
        }).toEqual({
            workers: [[true, "text/javascript"]],
            referenced: ["BAD_REQUEST", "ASSET_INVALID"],
        });
    } finally {
        await files.remove(directory, { recursive: true });
    }
});

test("run the browser baseline first in a worker, leaving a library's entries alone", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.makeTemporaryDirectory("build-baseline-");
    const source = join(directory, "source");
    const browser = {
        directory: source,
        dependencies: {},
        links: {},
        outputs: { library: { ...request.outputs.library, runtime: "browser" } },
    } as const;

    try {
        // start an echo worker from the library's note module
        await files.copy(
            fileURLToPath(new URL("../../tests/fixture/library/source/", import.meta.url)),
            source,
        );
        await files.writeTextFile(
            join(source, "src/echo.worker.ts"),
            'self.addEventListener("message", (event) => self.postMessage(event.data));\n',
        );
        const notePath = join(source, "src/note.ts");
        await files.writeTextFile(
            notePath,
            `${await files.readTextFile(notePath)}\n/** Start the echo worker. */\nexport const echo = () =>\n    new Worker(new URL("./echo.worker.ts", import.meta.url), { type: "module" });\n`,
        );
        await using store = await LocalBuildStore.temporary(environment);
        await using builder = await Compiler.start(source, {
            store,
            toolchain: async () => Toolchain.workspace(),
        });
        const built = await builder.build(browser);

        // read each emitted entry's code with the code of the modules it imports first
        const scripts = (await built.files()).filter((file) => file.path.endsWith(".js"));
        const texts = new Map<string, string>();
        for (const file of scripts) {
            texts.set(file.path, new TextDecoder().decode(await built.load(file.path)));
        }

        // expect the baseline's code to run before the worker's own, and in no library entry
        expect({
            note: runsBaselineFirst(texts, "output/library/src/note.js", "createNote"),
            index: runsBaselineFirst(texts, "output/library/src/index.js", "createNote"),
            worker: runsBaselineFirst(
                texts,
                [...texts.keys()].find((path) => /echo\.worker[^/]*\.js$/u.test(path)) ?? "",
                "addEventListener",
            ),
        }).toEqual({ note: false, index: false, worker: true });
    } finally {
        await files.remove(directory, { recursive: true });
    }
});

/** Decide whether a built script runs the baseline before its own code. */
function runsBaselineFirst(texts: ReadonlyMap<string, string>, path: string, own: string): boolean {
    // follow the script's first import or re-export to the module that runs first
    const text = texts.get(path) ?? "";
    const first = /^(?:import|export)[^"']*["'](\.{1,2}\/[^"']+)["']/mu.exec(text)?.[1];
    const running = first === undefined ? text : (texts.get(join(dirname(path), first)) ?? "");

    // find the baseline's data in it before the code it runs as its own
    const marker = running.indexOf("javascript.builtins.Set.union");

    return marker !== -1 && marker < running.lastIndexOf(own);
}
