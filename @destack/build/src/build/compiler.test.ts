import { expect, onTestFinished, test, vi } from "@destack/test";
import { cp, mkdir, mkdtemp, readFile, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { PackageBuild, Compiler } from "../index.ts";
import type { PackageManifest } from "@destack/package/manifest";
import { PackageError } from "@destack/package/error";
import { request } from "../../tests/fixture/library/request.ts";
import {
    countObjects,
    expectFiles,
    importBuilt,
    isNoteModule,
    type NoteModule,
} from "../../tests/fixture.ts";
import { formatSource } from "@destack/check";
import { Commit, Digest, present } from "@destack/schema";
import { LocalPackageStore } from "../store/local.ts";
import { NpmInstallation } from "../dependency/npm.ts";
import { convertHere, copy, writeStore } from "../dependency/test/store.ts";

/** How long a cancelled build runs before it aborts: well inside the library's build. */
const CANCEL_MILLISECONDS = 100;

test("rebuild edited source, reject incompatible APIs, restore distributed output", async () => {
    const fixture = new URL("../../tests/fixture/library/", import.meta.url);
    const directory = await mkdtemp(join(tmpdir(), "destack-build-library-"));
    const source = join(directory, "source");
    const destination = join(directory, "build");
    const library = { ...request, directory: source };

    try {
        // compile a real package
        await cp(new URL("source/", fixture), source, { recursive: true });
        await using store = await LocalPackageStore.temporary();
        await using builder = await Compiler.start(source, { store });
        await using build = await builder.build(library);

        // rebuild unchanged, edited, and restored source with the same compiler
        await using unchanged = await builder.build(library);
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
        await using edited = await builder.build(library);
        const editDirectory = join(directory, "edited");
        await PackageBuild.write(edited.reader, editDirectory);
        const editedModule = await importNotes(
            editDirectory,
            rootExport(edited.manifest, "library"),
        );
        expect(editedModule.createNote("Edited")).toEqual({ title: "Edited", complete: true });
        await writeFile(notePath, note);
        await using restoredSource = await builder.build(library);
        expect(restoredSource.manifest).toEqual(build.manifest);
        await expectFiles(restoredSource, build);

        // refuse a browser API in the workerd output, though the package also ships the module to the browser
        const workerdRequest = {
            directory: source,
            dependencies: {},
            outputs: { library: { ...request.outputs.library, runtime: "workerd" } },
        } as const;
        await writeFile(
            notePath,
            note + "\n/** The page title. */\nexport const page = document.title;\n",
        );
        await expect(builder.build(workerdRequest)).rejects.toMatchObject({
            code: "BUILD_FAILED",
            message: `unsupported workerd API: document.title at src/note.ts:${note.length + 44}`,
        });

        // accept the browser API inside a function, which only the browser calls
        const deferredSource =
            note + "\n/** Read the page title. */\nexport const page = () => document.title;\n";
        await writeFile(notePath, deferredSource);
        await using deferred = await builder.build(workerdRequest);
        expect(await readFile(join(deferred.directory, "src/note.ts"), "utf8")).toBe(
            deferredSource,
        );

        // refuse the Function constructor on workerd, and accept reads of its prototype
        const constructed =
            note +
            "\n/** The constructor. */\nexport const create: FunctionConstructor = Function;\n";
        await writeFile(notePath, constructed);
        await expect(
            builder.build({
                directory: source,
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
            directory: source,
            dependencies: {},
            outputs: { library: { ...request.outputs.library, runtime: "workerd" } },
        });
        expect(await readFile(join(workerd.directory, "src/note.ts"), "utf8")).toBe(
            prototypeSource,
        );

        // accept a global read by a well-known symbol, and refuse one read by a computed key
        const symbolSource =
            note + "\n/** The tag of Math. */\nexport const tag = Math[Symbol.toStringTag];\n";
        await writeFile(notePath, symbolSource);
        await using symbolKeyed = await builder.build(workerdRequest);
        expect(await readFile(join(symbolKeyed.directory, "src/note.ts"), "utf8")).toBe(
            symbolSource,
        );
        await writeFile(
            notePath,
            note +
                '\n/** A Math member. */\nexport const member = (key: string) => Math[key as "PI"];\n',
        );
        await expect(builder.build(workerdRequest)).rejects.toMatchObject({
            code: "BUILD_FAILED",
            message: `dynamic global access: Math at src/note.ts:${note.length + 62}`,
        });

        // accept browser index signatures through the runtime's actual type declarations
        const browserSource =
            note +
            '\n/** The page theme. */\nexport const page = document.documentElement.dataset["theme"];\n';
        await writeFile(notePath, browserSource);
        await using browser = await builder.build({
            directory: source,
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
        await PackageBuild.write(build.reader, destination);
        await rm(source, { recursive: true });
        const restored = await PackageBuild.read(destination);
        expect(restored.manifest).toEqual(build.manifest);

        // preserve an existing distribution when the caller repeats a write
        await expect(PackageBuild.write(build.reader, destination)).rejects.toMatchObject({
            code: "EEXIST",
        });
        expect((await PackageBuild.read(destination)).manifest).toEqual(build.manifest);

        // refuse writing a file whose bytes of the same size no longer match its digest
        const changed = present(
            (await build.reader.distributed()).find(
                (file) => file.path !== rootExport(build.manifest, "library"),
            ),
            "a distributed file besides the library",
        );
        const original = await readFile(join(build.directory, changed.path));
        const flipped = Uint8Array.from(original, (byte, index) => (index === 0 ? byte ^ 1 : byte));
        await writeFile(join(build.directory, changed.path), flipped);
        const corrupted = join(directory, "corrupted");
        await expect(PackageBuild.write(build.reader, corrupted)).rejects.toEqual(
            new PackageError("INVALID_FILE", `file digest mismatch: ${changed.path}`),
        );
        await writeFile(join(build.directory, changed.path), original);

        // leave no published manifest when an output cannot be written
        const failed = join(directory, "failed");
        await rm(join(build.directory, rootExport(build.manifest, "library")));
        await expect(PackageBuild.write(build.reader, failed)).rejects.toMatchObject({
            code: "ENOENT",
        });
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
        await using store = await LocalPackageStore.temporary();
        await using builder = await Compiler.start(source, { store });
        const library = { ...request, directory: source };
        await using first = await builder.build(library);
        expect(first.reused).toEqual([]);

        // restore the whole unchanged build from the cache, file for file, in this compiler and another
        await using unchanged = await builder.build(library);
        expect(unchanged.reused).toEqual(["library"]);
        expect(unchanged.manifest).toEqual(first.manifest);
        await expectFiles(unchanged, first);
        await using other = await Compiler.start(source, { store });
        await using elsewhere = await other.build(library);
        expect(elsewhere.reused).toEqual(["library"]);

        // reuse the library output beside a module it does not import, as a build without the cache compiles it
        const draft = join(source, "src/draft.ts");
        await writeFile(
            draft,
            await formatSource(draft, '/** A draft title. */\nexport const draft = "draft";\n'),
        );
        await using added = await builder.build(library);
        await using empty = await LocalPackageStore.temporary();
        await using uncached = await Compiler.start(source, { store: empty });
        await using cold = await uncached.build(library);
        expect(added.reused).toEqual(["library"]);
        expect(added.manifest).toEqual(cold.manifest);
        await expectFiles(added, cold);

        // compile the library again once a module it imports changes
        const note = join(source, "src/note.ts");
        const original = await readFile(note, "utf8");
        await writeFile(note, original.replace("complete: false", "complete: true"));
        await using edited = await builder.build(library);
        expect(edited.reused).toEqual([]);
        expect(edited.manifest).not.toEqual(added.manifest);

        // restore the earlier build from the cache once the module is restored
        await writeFile(note, original);
        await using restored = await builder.build(library);
        expect(restored.reused).toEqual(["library"]);
        expect(restored.manifest).toEqual(added.manifest);
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("reuse cached outputs sharing source files once the build's commit changes", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-build-shared-"));
    const source = join(directory, "source");
    try {
        // build the library for two runtimes into a store's cache at one commit
        await cp(new URL("../../tests/fixture/library/source/", import.meta.url), source, {
            recursive: true,
        });
        await using store = await LocalPackageStore.temporary();
        await using builder = await Compiler.start(source, { store });
        const outputs = {
            bun: request.outputs.library,
            workerd: { ...request.outputs.library, runtime: "workerd" },
        } as const;
        const shared = { directory: source, dependencies: {}, outputs };
        await using first = await builder.build({
            ...shared,
            commit: Commit.parse("a".repeat(40)),
        });
        expect(first.reused).toEqual([]);

        // reuse both outputs at another commit, as a build without the cache compiles them
        const commit = Commit.parse("b".repeat(40));
        await using reused = await builder.build({ ...shared, commit });
        await using empty = await LocalPackageStore.temporary();
        await using uncached = await Compiler.start(source, { store: empty });
        await using cold = await uncached.build({ ...shared, commit });
        expect(reused.reused).toEqual(["bun", "workerd"]);
        expect(reused.manifest).toEqual(cold.manifest);
        await expectFiles(reused, cold);
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("reject invalid outputs and recover the retained compiler", async () => {
    // build once before any rejected request
    const source = fileURLToPath(new URL("../../tests/fixture/library/source/", import.meta.url));
    const library = { ...request, directory: source };
    await using store = await LocalPackageStore.temporary();
    await using builder = await Compiler.start(source, { store });
    await using first = await builder.build(library);
    await expect(
        builder.build({ directory: source, dependencies: {}, outputs: {} }),
    ).rejects.toMatchObject({
        code: "BUILD_FAILED",
        message: "a build requires at least one output",
    });
    await expect(
        builder.build({
            directory: source,
            dependencies: {},
            outputs: { website: { kind: "web", app: "src/app.tsx", ssr: false } },
        }),
    ).rejects.toMatchObject({
        code: "BUILD_FAILED",
        message: "no dependency compiles outputs of kind web",
    });

    // build the same package again with the same compiler after rejected requests
    await using build = await builder.build(library);
    expect(build.manifest).toEqual(first.manifest);
});

test("terminate compilation on cancellation and deadline, and build again with the same builder", async () => {
    const directory = fileURLToPath(
        new URL("../../tests/fixture/library/source/", import.meta.url),
    );
    const library = { ...request, directory };
    const controller = new AbortController();
    await using store = await LocalPackageStore.temporary();
    await using builder = await Compiler.start(directory, { store });

    // cancel a build once the compiler has it, as the library's build takes over a second
    const pending = builder.build({ ...library, signal: controller.signal });
    setTimeout(() => controller.abort(), CANCEL_MILLISECONDS);
    await expect(pending).rejects.toMatchObject({
        code: "BUILD_FAILED",
        message: "build cancelled",
    });

    // stop a compiler past its deadline, and build again with the same builder
    await expect(builder.build({ ...library, timeout: 1 })).rejects.toMatchObject({
        code: "BUILD_FAILED",
        message: "build exceeded 1 ms",
    });
    await using rebuilt = await builder.build(library);
    expect(rebuilt.manifest.package.name).toBe("@destack/build-library-fixture");
});

test("close a compiler once it exits, however long its shutdown takes", async () => {
    // start a compiler that exits only once the test releases it, after the end of its input
    const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-shutdown-")));
    const release = join(directory, "release");
    const script = join(directory, "compiler.ts");
    await writeFile(
        script,
        `for await (const _chunk of process.stdin) {}
while (!(await Bun.file(${JSON.stringify(release)}).exists())) {
    await Bun.sleep(10);
}
`,
    );
    const source = fileURLToPath(new URL("../../tests/fixture/library/source/", import.meta.url));
    await using store = await LocalPackageStore.temporary();
    const builder = await Compiler.start(source, {
        command: {
            executable: process.execPath,
            arguments: ["run", "--no-env-file", script],
            read: [directory],
        },
        store,
    });

    // stay open an hour past the end of its input, then close once it exits
    try {
        vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
        let isClosed = false;
        const closing = builder[Symbol.asyncDispose]().then(() => {
            isClosed = true;
        });
        vi.advanceTimersByTime(3_600_000);
        vi.useRealTimers();
        await new Promise((resolve) => {
            setImmediate(resolve);
        });
        const isClosedEarly = isClosed;
        await writeFile(release, "");
        await closing;
        expect([isClosedEarly, isClosed]).toEqual([false, true]);
    } finally {
        vi.useRealTimers();
        await rm(directory, { recursive: true });
    }
});

test("compile a package with the build extension it declares", async () => {
    const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-build-extension-")));
    try {
        // write a package whose module reads a constant only its extension writes in
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
        await using store = await LocalPackageStore.temporary();
        await using builder = await Compiler.start(directory, { store });
        const outputs = { library: { kind: "module", runtime: "bun" } } as const;
        await using built = await builder.build({
            directory: directory,
            dependencies: {},
            outputs,
        });
        await PackageBuild.write(built.reader, join(directory, "build"));
        const module = await importBuilt(
            join(directory, "build"),
            rootExport(built.manifest, "library"),
        );
        expect({ ...module }).toEqual({ greeting: "hello" });
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("refuse a workerd output importing an npm unit that workerd cannot load", async () => {
    // store an npm unit of a native addon, which has no workerd output
    const { root, locked } = await writeStore({
        native: {
            manifest: { main: "index.js" },
            files: { "index.js": "module.exports = 1;\n", "addon.node": "" },
        },
    });
    await using store = await LocalPackageStore.temporary();
    const installation = new NpmInstallation(
        locked,
        store,
        await Digest.json({ test: "toolchain" }),
    );
    const native = await installation.store(
        await installation.open(copy(root, "native")),
        convertHere,
    );

    // write a workerd package importing it
    const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-build-addon-")));
    onTestFinished(() => rm(directory, { recursive: true }));
    const files: Record<string, string> = {
        "bun.lock": JSON.stringify({ lockfileVersion: 1, workspaces: {}, packages: {} }),
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
    for (const [path, text] of Object.entries(files)) {
        await mkdir(join(directory, path, ".."), { recursive: true });
        await writeFile(join(directory, path), text);
    }
    await cp(copy(root, "native"), join(directory, "node_modules", "native"), { recursive: true });

    // refuse its build before the store stores it
    const stored = await countObjects(store);
    await using builder = await Compiler.start(directory, { store });
    const built = builder.build({
        directory,
        dependencies: { native },
        outputs: { server: { kind: "module", runtime: "workerd" } },
    });
    await expect(built).rejects.toMatchObject({
        code: "BUILD_FAILED",
        message: "the workerd output of @example/addon needs native, which workerd cannot load",
    });
    expect(await countObjects(store)).toBe(stored);
});

test("check every output's entries with the package's ambient declarations", async () => {
    // write a package for Bun and browsers whose entry uses a type its ambient declarations declare
    const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-build-ambient-")));
    onTestFinished(() => rm(directory, { recursive: true }));
    const files: Record<string, string> = {
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
    for (const [path, text] of Object.entries(files)) {
        await mkdir(join(directory, path, ".."), { recursive: true });
        await writeFile(join(directory, path), text);
    }

    // build both outputs, the browser's program rooted at its entries
    await using store = await LocalPackageStore.temporary();
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
    const path = manifest.outputs[output]?.exports["."];
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
