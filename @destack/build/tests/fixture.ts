import type { Environment, FileSystem } from "@destack/environment";
import { BuildReader } from "@destack/package/manifest";
import { found } from "@destack/schema";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { Buffer } from "node:buffer";
import { expect } from "@destack/test";
import type { graph } from "@destack/package";
import { type PackageBuildOptions, Compiler, type PackageBuild } from "../src/index.ts";
import { comparePath } from "../src/build/serialization.ts";
import { LocalPackageStore } from "../src/store/local.ts";
import { linkDependencies } from "../src/source/index.ts";
import { copySource } from "../src/test/source.ts";

/** The kind package's constructor, inspector and manifests, and the app declaring one thing. */
export const THING_FILES: Readonly<Record<string, string>> = {
    "bun.lock": JSON.stringify({ lockfileVersion: 2, workspaces: {}, packages: {} }),
    "package.json": JSON.stringify({
        name: "@example/app",
        version: "2026.9.0",
        type: "module",
        exports: { ".": "./src/app.ts" },
        dependencies: { "@example/owner": "2026.9.0" },
    }),
    "destack.json": JSON.stringify({
        $schema: "https://destack.app/schemas/2026.9.0/destack.json",
        id: "package-01996ab0-0000-7000-8000-00000000000c",
        language: "typescript",
        runtimes: ["bun"],
    }),
    "src/app.ts":
        'import { defineThing } from "@example/owner";\n\nexport const greeting = defineThing({ name: "greeting" });\n',
    "node_modules/@example/owner/package.json": JSON.stringify({
        name: "@example/owner",
        version: "2026.9.0",
        type: "module",
        exports: { ".": "./index.ts", "./inspect": "./inspect.ts" },
    }),
    "node_modules/@example/owner/destack.json": JSON.stringify({
        $schema: "https://destack.app/schemas/2026.9.0/destack.json",
        id: "package-01996ab0-0000-7000-8000-00000000000a",
        language: "typescript",
        runtimes: ["bun"],
        declarations: {
            defineThing: { describes: [{ kind: "thing", function: "./inspect#describe" }] },
        },
        stamps: { defineThing: { module: 1 } },
    }),
    "node_modules/@example/owner/index.ts":
        "export function defineThing(definition: { name: string }, module?: { package: object }) {\n    return { ...definition, package: module!.package };\n}\n",
    "node_modules/@example/owner/inspect.ts":
        "export function describe(value: { name: string }) {\n    return { name: value.name };\n}\n",
};

/** Write the thing kind's package and an app as a git workspace, answering the app. */
export async function writeThing(
    directory: string,
    app: string,
    environment: Environment<"files" | "process">,
): Promise<string> {
    // place the kind's package as a member beside the app, which links it
    const { files } = environment;
    const owner = "node_modules/@example/owner/";
    const written: Record<string, string> = {
        "bun.lock": JSON.stringify({
            lockfileVersion: 2,
            workspaces: { owner: { name: "@example/owner" }, app: { name: "@example/app" } },
            packages: {},
        }),
        ".gitignore": "node_modules/\n",
    };
    for (const [path, text] of Object.entries({ ...THING_FILES, "src/app.ts": app })) {
        if (path !== "bun.lock") {
            written[path.startsWith(owner) ? `owner/${path.slice(owner.length)}` : `app/${path}`] =
                text;
        }
    }
    for (const [path, text] of Object.entries(written)) {
        await files.mkdir(dirname(join(directory, path)), { recursive: true });
        await files.writeTextFile(join(directory, path), text);
    }
    await files.mkdir(join(directory, "app/node_modules/@example"), { recursive: true });
    await files.symlink(
        join(directory, "owner"),
        join(directory, "app/node_modules/@example/owner"),
    );
    const initialized = await environment.process
        .spawn(["git", "init", "--quiet", directory])
        .output();
    expect(initialized.code).toBe(0);

    return join(directory, "app");
}

/** A built library module exporting createNote. */
export interface NoteModule {
    /** Create a note with a title. */
    readonly createNote: (title: string) => unknown;
}

/** Read the file a build output exports at a path, refusing an output or export the build lacks. */
export function exported(
    output: { readonly exports: Readonly<Record<string, string>> } | undefined,
    path: string,
): string {
    const file = output?.exports[path];
    if (file === undefined) {
        throw new TypeError(`the build output exports no ${path}`);
    }

    return file;
}

/** Compare a complete file collection with its expected directory. */
export async function expectDirectory(
    written: ReadonlyMap<string, Uint8Array>,
    expected: URL,
    environment: Environment<"files" | "process">,
): Promise<void> {
    const { files } = environment;
    const directory = fileURLToPath(expected);
    const paths = [...written.keys()].toSorted();
    const isUpdating = environment.process.variables["UPDATE_BUILD_FIXTURES"] === "1";

    // refresh only the explicitly selected fixture output
    if (isUpdating) {
        await files.mkdir(directory, { recursive: true });
        const entries = await files.readDirectory(directory, { recursive: true });
        for (const entry of entries) {
            if (entry.kind === "file" && !written.has(entry.name.replaceAll("\\", "/"))) {
                await files.remove(join(directory, entry.name));
            }
        }
        for (const [path, bytes] of written) {
            const destination = join(directory, path);
            await files.mkdir(dirname(destination), { recursive: true });
            await files.writeFile(destination, bytes);
        }
    }

    // reject missing and unexpected files before comparing their contents
    const entries = await files.readDirectory(directory, { recursive: true });
    const actual = entries
        .filter((entry) => entry.kind === "file")
        .map((entry) => entry.name.replaceAll("\\", "/"))
        .toSorted();
    expect(actual).toEqual(paths);
    for (const path of paths) {
        const bytes = await files.readFile(join(directory, path));
        const expectedBytes = found(written, path);
        if (Buffer.compare(bytes, expectedBytes) !== 0) {
            expect(bytes, path).toEqual(expectedBytes);
        }
    }
}

/** Compare complete file sets without traversing each byte as an object property. */
export async function expectFiles(
    actualBuild: PackageBuild,
    expectedBuild: PackageBuild,
    files: FileSystem,
): Promise<void> {
    const actual = await readBuildFiles(actualBuild, files);
    const expected = await readBuildFiles(expectedBuild, files);
    expect([...actual.keys()]).toEqual([...expected.keys()]);
    for (const [path, bytes] of actual) {
        expect(Buffer.compare(bytes, found(expected, path)), path).toBe(0);
    }
}

/** Read fixture outputs for complete byte comparisons. */
export async function readBuildFiles(
    build: PackageBuild,
    files: FileSystem,
): Promise<Map<string, Uint8Array<ArrayBuffer>>> {
    const read = new Map<string, Uint8Array<ArrayBuffer>>();
    const records = (await build.reader.distributed()).toSorted(comparePath);
    for (const file of records) {
        read.set(file.path, await files.readFile(join(build.directory, file.path)));
    }

    return read;
}

/** An isolated source checkout of a build fixture without Destack or npm dependencies. */
export class Fixture implements AsyncDisposable {
    /** Source fixture location. */
    readonly fixture: URL;
    /** Temporary fixture directory. */
    readonly directory: string;
    /** Copied package source directory. */
    readonly source: string;
    /** The store the fixture's compiler builds into. */
    readonly #store: LocalPackageStore;
    /** The compiler building and indexing the copy. */
    readonly #compiler: Compiler;
    /** The file system the checkout lives on. */
    readonly #files: FileSystem;

    /** Retain the fixture, its copied source, and the store and compiler building it. */
    private constructor(
        fixture: URL,
        directory: string,
        source: string,
        built: { readonly store: LocalPackageStore; readonly compiler: Compiler },
        files: FileSystem,
    ) {
        this.fixture = fixture;
        this.directory = directory;
        this.source = source;
        this.#store = built.store;
        this.#compiler = built.compiler;
        this.#files = files;
    }

    /** Copy a fixture into a temporary checkout beside the installed toolchain. */
    static async open(
        name: string,
        environment: Environment<"files" | "process" | "host">,
    ): Promise<Fixture> {
        // copy each case into a separate editable checkout
        const { files } = environment;
        const fixture = new URL(`./fixture/${name}/`, import.meta.url);
        const directory = await files.makeTemporaryDirectory("destack-build-fixture-");
        const source = join(directory, "source");
        try {
            await copySource(fileURLToPath(new URL("source/", fixture)), source, files);
            await linkDependencies(fileURLToPath(new URL("source/", fixture)), source, files);

            const store = await LocalPackageStore.temporary(environment);
            const compiler = await Compiler.start(source, { store });

            return new Fixture(fixture, directory, source, { store, compiler }, files);
        } catch (error) {
            await files.remove(directory, { recursive: true });
            throw error;
        }
    }

    /** Build the copied package with the fixture's output selection. */
    async build(
        request: Omit<PackageBuildOptions, "directory" | "dependencies">,
    ): Promise<PackageBuild> {
        return await this.#compiler.build({ directory: this.source, dependencies: {}, ...request });
    }

    /** Index the copied package's code for Bun. */
    async index(): Promise<graph.Document[]> {
        return await this.#compiler.index({
            directory: this.source,
            runtime: "bun",
            dependencies: {},
        });
    }

    /** Close the compiler and the store, then remove the fixture's temporary checkout. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#compiler[Symbol.asyncDispose]();
        await this.#store[Symbol.asyncDispose]();
        await this.#files.remove(this.directory, { recursive: true });
    }
}

/** Read each manifest list and graph file independently from a relocated package. */
export async function expectManifest(
    build: PackageBuild,
    destination: string,
    environment: Environment<"files">,
): Promise<void> {
    // load each file independently without touching executable files
    const { files } = environment;
    const loaded: string[] = [];
    const reader = new BuildReader(
        build.manifest,
        async (path) => {
            loaded.push(path);

            return new Blob([await files.readFile(join(destination, path))]).stream();
        },
        environment.crypto,
    );

    // read each list independently and compare its complete serialized contents
    for (const [name, read] of [
        ["dependencies", () => reader.dependencies()],
        ["files", () => reader.files()],
        ["sourceMaps", () => reader.sourceMaps()],
    ] as const) {
        loaded.length = 0;
        const records = await read();
        const reference = build.manifest.lists[name];
        const expected: unknown = JSON.parse(
            await files.readTextFile(join(build.directory, reference.path)),
        );
        expect(records).toEqual(expected);
        expect(loaded).toEqual([reference.path]);
    }

    // load the graph root alone, then each module's graph file alone by its digest
    loaded.length = 0;
    const root = await reader.graph();
    expect(loaded).toEqual([build.manifest.lists.graph.path]);
    for (const [path, digest] of Object.entries(root.modules)) {
        loaded.length = 0;
        const module = await reader.module(digest);
        expect([module.path, loaded]).toEqual([path, [`graph/${digest}.json`]]);
    }
}

/** Import a module of a written build by its package path. */
export async function importBuilt(destination: string, path: string): Promise<object> {
    const module: unknown = await import(pathToFileURL(join(destination, path)).href);
    if (typeof module !== "object" || module === null) {
        throw new TypeError(`${path} is no module`);
    }

    return module;
}

/** Read a property of a built object, undefined when absent. */
export function property(value: unknown, name: string): unknown {
    if (typeof value !== "object" || value === null) {
        throw new TypeError(`the built value holds no object with ${name}`);
    }
    const read: unknown = Reflect.get(value, name);

    return read;
}

/** Report whether a module exports createNote. */
export function isNoteModule(module: object): module is NoteModule {
    return "createNote" in module && typeof module.createNote === "function";
}

/** Count the objects a store keeps. */
export async function countObjects(store: LocalPackageStore): Promise<number> {
    // count each page of the bucket's listing
    let count = 0;
    let cursor: string | undefined;
    do {
        const page = await store.bucket.list(cursor === undefined ? {} : { cursor });
        count += page.files.length;
        cursor = page.cursor;
    } while (cursor !== undefined);

    return count;
}
