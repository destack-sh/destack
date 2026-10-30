import { yieldToEventLoop } from "@destack/environment";
import { testSimulation } from "../test/world.ts";
import { expect, refusal, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { PackageBuild, Compiler } from "../index.ts";
import type { PackageManifest } from "@destack/package/manifest";
import { request } from "../../tests/fixture/library/request.ts";
import {
    countObjects,
    expectFiles,
    importBuilt,
    isNoteModule,
    type NoteModule,
} from "../../tests/fixture.ts";
import { present } from "@destack/schema";
import { LocalPackageStore } from "../store/local.ts";
import { copy, writeStore } from "../dependency/test/store.ts";

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
        await using store = await LocalPackageStore.temporary(environment);
        await using builder = await Compiler.start(source, { store });
        await using build = await builder.build(library);

        // rebuild unchanged, edited, and restored source with the same compiler
        await using unchanged = await builder.build(library);
        expect(unchanged.manifest).toEqual(build.manifest);
        await expectFiles(unchanged, build, files);
        const notePath = join(source, "src/note.ts");
        const note = await files.readTextFile(notePath);
        await files.writeTextFile(
            notePath,
            note
                .replace("complete: boolean", "complete: true")
                .replace("complete: false", "complete: true"),
        );
        await using edited = await builder.build(library);
        const editDirectory = join(directory, "edited");
        await edited.reader.writeDirectory(editDirectory, files);
        const editedModule = await importNotes(
            editDirectory,
            rootExport(edited.manifest, "library"),
        );
        expect(editedModule.createNote("Edited")).toEqual({ title: "Edited", complete: true });
        await files.writeTextFile(notePath, note);
        await using restoredSource = await builder.build(library);
        expect(restoredSource.manifest).toEqual(build.manifest);
        await expectFiles(restoredSource, build, files);

        // refuse a browser API in the checked workerd output, though the package also ships the module to the browser
        const workerdRequest = {
            directory: source,
            dependencies: {},
            outputs: { library: { ...request.outputs.library, runtime: "workerd" } },
        } as const;
        await files.writeTextFile(
            notePath,
            note + "\n/** The page title. */\nexport const page = document.title;\n",
        );
        expect(await refusal(builder.check(workerdRequest))).toEqual([
            "UNPROCESSABLE_CONTENT",
            "RUNTIME_API_UNSUPPORTED",
            `unsupported workerd API: document.title at src/note.ts:${note.length + 44}`,
        ]);

        // accept the browser API inside a function, which only the browser calls
        const deferredSource =
            note + "\n/** Read the page title. */\nexport const page = () => document.title;\n";
        await files.writeTextFile(notePath, deferredSource);
        await using deferred = await builder.check(workerdRequest);
        expect(await files.readTextFile(join(deferred.directory, "src/note.ts"))).toBe(
            deferredSource,
        );

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
                    outputs: { library: { ...request.outputs.library, runtime: "workerd" } },
                }),
            ),
        ).toEqual([
            "UNPROCESSABLE_CONTENT",
            "RUNTIME_API_UNSUPPORTED",
            `unsupported workerd API: Function at src/note.ts:${note.length + 68}`,
        ]);
        const prototypeSource =
            note +
            "\n/** The function prototype's names. */\nexport const names = Object.getOwnPropertyNames(Function.prototype);\n";
        await files.writeTextFile(notePath, prototypeSource);
        await using workerd = await builder.check({
            directory: source,
            dependencies: {},
            outputs: { library: { ...request.outputs.library, runtime: "workerd" } },
        });
        expect(await files.readTextFile(join(workerd.directory, "src/note.ts"))).toBe(
            prototypeSource,
        );

        // accept a global read by a well-known symbol, and refuse one read by a computed key
        const symbolSource =
            note + "\n/** The tag of Math. */\nexport const tag = Math[Symbol.toStringTag];\n";
        await files.writeTextFile(notePath, symbolSource);
        await using symbolKeyed = await builder.check(workerdRequest);
        expect(await files.readTextFile(join(symbolKeyed.directory, "src/note.ts"))).toBe(
            symbolSource,
        );
        await files.writeTextFile(
            notePath,
            note +
                '\n/** A Math member. */\nexport const member = (key: string) => Math[key as "PI"];\n',
        );
        expect(await refusal(builder.check(workerdRequest))).toEqual([
            "UNPROCESSABLE_CONTENT",
            "RUNTIME_API_UNSUPPORTED",
            `dynamic global access: Math at src/note.ts:${note.length + 62}`,
        ]);

        // accept browser index signatures through the runtime's actual type declarations
        const browserSource =
            note +
            '\n/** Read the page theme. */\nexport const page = () => document.documentElement.dataset["theme"];\n';
        await files.writeTextFile(notePath, browserSource);
        await using browser = await builder.check({
            directory: source,
            dependencies: {},
            outputs: {
                library: {
                    ...request.outputs.library,
                    runtime: "browser",
                },
            },
        });
        expect(await files.readTextFile(join(browser.directory, "src/note.ts"))).toBe(
            browserSource,
        );

        // remove source access before loading the distributed package
        await build.reader.writeDirectory(destination, files);
        await files.remove(source, { recursive: true });
        const restored = await PackageBuild.read(destination, environment);
        expect(restored.manifest).toEqual(build.manifest);

        // preserve an existing distribution when the caller repeats a write
        expect(await refusal(build.reader.writeDirectory(destination, files))).toEqual([
            "CONFLICT",
            "ALREADY_EXISTS",
            `cannot create ${destination}: EEXIST`,
        ]);
        expect((await PackageBuild.read(destination, environment)).manifest).toEqual(
            build.manifest,
        );

        // refuse writing a file whose bytes of the same size no longer match its digest
        const changed = present(
            (await build.reader.distributed()).find(
                (file) => file.path !== rootExport(build.manifest, "library"),
            ),
            "a distributed file besides the library",
        );
        const original = await files.readFile(join(build.directory, changed.path));
        const flipped = Uint8Array.from(original, (byte, index) => (index === 0 ? byte ^ 1 : byte));
        await files.writeFile(join(build.directory, changed.path), flipped);
        const corrupted = join(directory, "corrupted");
        expect(await refusal(build.reader.writeDirectory(corrupted, files))).toEqual([
            "BAD_REQUEST",
            "FILE_INVALID",
            `file digest mismatch: ${changed.path}`,
        ]);
        await files.writeFile(join(build.directory, changed.path), original);

        // leave no published manifest when an output cannot be written
        const failed = join(directory, "failed");
        await files.remove(join(build.directory, rootExport(build.manifest, "library")));
        const removed = join(build.directory, rootExport(build.manifest, "library"));
        expect(await refusal(build.reader.writeDirectory(failed, files))).toEqual([
            "NOT_FOUND",
            "NOT_FOUND",
            `cannot open ${removed}: ENOENT`,
        ]);
        expect(await refusal(files.readFile(join(failed, "manifest.json")))).toEqual([
            "NOT_FOUND",
            "NOT_FOUND",
            `cannot read ${join(failed, "manifest.json")}: ENOENT`,
        ]);

        const module = await importNotes(destination, rootExport(restored.manifest, "library"));
        expect(module.createNote("A note")).toEqual({ title: "A note", complete: false });

        // discard temporary compiler output while preserving the retained distribution
        await build[Symbol.asyncDispose]();
        const written = join(build.directory, "manifest.json");
        expect(await refusal(files.readFile(written))).toEqual([
            "NOT_FOUND",
            "NOT_FOUND",
            `cannot read ${written}: ENOENT`,
        ]);
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
    await using store = await LocalPackageStore.temporary(environment);
    await using builder = await Compiler.start(source, { store });
    await using first = await builder.build(library);
    expect(
        await refusal(builder.build({ directory: source, dependencies: {}, outputs: {} })),
    ).toEqual(["BAD_REQUEST", "OUTPUT_INVALID", "a build requires at least one output"]);
    expect(
        await refusal(
            builder.build({
                directory: source,
                dependencies: {},
                outputs: { website: { kind: "web", app: "src/app.tsx", ssr: false } },
            }),
        ),
    ).toEqual(["BAD_REQUEST", "OUTPUT_INVALID", "no dependency compiles outputs of kind web"]);

    // build the same package again with the same compiler after rejected requests
    await using build = await builder.build(library);
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
    await using store = await LocalPackageStore.temporary(environment);
    await using builder = await Compiler.start(directory, { store });

    // cancel a build once the compiler has it, as the library's build takes over a second
    const pending = builder.build({ ...library, signal: controller.signal });
    void environment.clock.sleep(CANCEL_MILLISECONDS).then(() => controller.abort());
    expect(await refusal(pending)).toEqual([
        "CLIENT_CLOSED_REQUEST",
        "BUILD_CANCELLED",
        "build cancelled",
    ]);

    // stop a compiler past its deadline, and build again with the same builder
    expect(await refusal(builder.build({ ...library, timeout: 1 }))).toEqual([
        "GATEWAY_TIMEOUT",
        "COMPILER_TIMEOUT",
        "build exceeded 1 ms",
    ]);
    await using rebuilt = await builder.build(library);
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
    await using store = await LocalPackageStore.temporary(environment);
    const builder = await Compiler.start(source, {
        command: {
            executable: environment.process.executable,
            arguments: ["run", "--no-env-file", script],
            read: [directory],
            variables: {},
        },
        store,
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
        await using store = await LocalPackageStore.temporary(environment);
        await using builder = await Compiler.start(directory, { store });
        const outputs = { library: { kind: "module", runtime: "bun" } } as const;
        await using built = await builder.build({
            directory: directory,
            dependencies: {},
            outputs,
        });
        await built.reader.writeDirectory(join(directory, "build"), files);
        const module = await importBuilt(
            join(directory, "build"),
            rootExport(built.manifest, "library"),
        );
        expect({ ...module }).toEqual({ greeting: "hello" });
    } finally {
        await files.remove(directory, { recursive: true });
    }
});

test("refuse a workerd output importing an npm unit that workerd cannot load", async () => {
    // install a native addon, which workerd cannot load
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const { root, locked } = await writeStore(
        {
            native: {
                manifest: { main: "index.js" },
                files: { "index.js": "module.exports = 1;\n", "addon.node": "" },
            },
        },
        files,
    );
    await using store = await LocalPackageStore.temporary(environment);
    const release = present(locked["native@1.0.0"], "the native release");

    // write a workerd package importing it, locked in its lockfile
    const directory = await files.realPath(
        await files.makeTemporaryDirectory("destack-build-addon-"),
    );
    const contents: Record<string, string> = {
        "bun.lock": JSON.stringify({
            lockfileVersion: 2,
            workspaces: {},
            packages: { native: ["native@1.0.0", "", {}, release.integrity] },
        }),
        "destack.json": JSON.stringify({
            $schema: "https://destack.app/schemas/2026.9.0/destack.json",
            id: "package-01996ab0-0000-7000-8000-00000000000f",
            language: "typescript",
            runtimes: ["workerd"],
        }),
        "package.json": JSON.stringify({
            name: "@example/addon",
            version: "2026.9.0",
            type: "module",
            dependencies: { native: "1.0.0" },
            exports: { ".": "./src/index.ts" },
        }),
        "src/native.d.ts":
            'declare module "native" {\n    const value: number;\n    export default value;\n}\n',
        "src/index.ts":
            'import value from "native";\n\n/** The addon\'s value, doubled. */\nexport const doubled: number = value * 2;\n',
    };
    for (const [path, text] of Object.entries(contents)) {
        await files.mkdir(join(directory, path, ".."), { recursive: true });
        await files.writeTextFile(join(directory, path), text);
    }
    await files.copy(copy(root, "native"), join(directory, "node_modules", "native"));

    // refuse its build before the store stores it
    const stored = await countObjects(store);
    await using builder = await Compiler.start(directory, { store });
    const built = builder.build({
        directory,
        dependencies: {},
        outputs: { server: { kind: "module", runtime: "workerd" } },
    });
    expect(await refusal(built)).toEqual([
        "UNPROCESSABLE_CONTENT",
        "RUNTIME_MODULE_UNSUPPORTED",
        "native is a native addon, which workerd cannot load",
    ]);
    expect(await countObjects(store)).toBe(stored);
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
    await using store = await LocalPackageStore.temporary(environment);
    await using builder = await Compiler.start(directory, { store });
    await using built = await builder.build({
        directory,
        dependencies: {},
        outputs: {
            server: { kind: "module", runtime: "bun" },
            browser: { kind: "module", runtime: "browser" },
        },
    });
    expect(Object.keys(built.manifest.outputs).toSorted()).toEqual(["browser", "server"]);
});

/** Read the file an output exports at its root, failing when the manifest lacks it. */
function rootExport(manifest: PackageManifest, output: string): string {
    const path = manifest.outputs[output]?.exports["."]?.path;
    if (path === undefined) {
        throw new Error(`the manifest has no root export of ${output}`);
    }

    return path;
}

/** Import a written build's library module, requiring its createNote function. */
async function importNotes(destination: string, path: string): Promise<NoteModule> {
    const module = await importBuilt(destination, path);
    if (!isNoteModule(module)) {
        throw new TypeError(`${path} exports no createNote function`);
    }

    return module;
}
