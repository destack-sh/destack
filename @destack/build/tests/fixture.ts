import { BuildReader } from "@destack/package/manifest";
import { found } from "@destack/schema";
import { cp, mkdtemp, mkdir, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, join, relative } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { Buffer } from "node:buffer";
import { expect } from "@destack/test";
import {
    buildPackage,
    readDependencies,
    type BuildOptions,
    type PackageBuild,
} from "../src/index.ts";
import { comparePath } from "../src/build/serialization.ts";
import { linkDependencies } from "../src/source/index.ts";
import { tmpdir } from "node:os";
import type { Service } from "@destack/service";
import type { Workload } from "@destack/service/workload";

/** A built library module exporting createNote. */
export interface NoteModule {
    /** Create a note with a title. */
    createNote(title: string): unknown;
}

/** A built server module whose default export answers requests. */
export interface HandlerModule {
    /** The request handler. */
    readonly default: { fetch(request: Request): Response | Promise<Response> };
}

/** A built service module exporting its workload and the service it serves. */
export interface ServiceModule {
    /** The workload starting the service. */
    readonly web: Workload;
    /** The service. */
    readonly service: Service;
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
    files: ReadonlyMap<string, Uint8Array>,
    expected: URL,
): Promise<void> {
    const directory = fileURLToPath(expected);
    const paths = [...files.keys()].toSorted();
    const isUpdating = process.env["UPDATE_BUILD_FIXTURES"] === "1";

    // refresh only the explicitly selected fixture output
    if (isUpdating) {
        await mkdir(directory, { recursive: true });
        const entries = await readdir(directory, { recursive: true, withFileTypes: true });
        for (const entry of entries) {
            if (!entry.isFile()) {
                continue;
            }
            const path = join(entry.parentPath, entry.name);
            if (!files.has(relative(directory, path).replaceAll("\\", "/"))) {
                await rm(path);
            }
        }
        for (const [path, bytes] of files) {
            const destination = join(directory, path);
            await mkdir(dirname(destination), { recursive: true });
            await writeFile(destination, bytes);
        }
    }

    // reject missing and unexpected files before comparing their contents
    const entries = await readdir(directory, { recursive: true, withFileTypes: true });
    const actual = entries
        .filter((entry) => entry.isFile())
        .map((entry) =>
            relative(directory, join(entry.parentPath, entry.name)).replaceAll("\\", "/"),
        )
        .toSorted();
    expect(actual).toEqual(paths);
    for (const path of paths) {
        const bytes = await readFile(join(directory, path));
        const written = found(files, path);
        if (Buffer.compare(bytes, written) !== 0) {
            expect(new Uint8Array(bytes), path).toEqual(written);
        }
    }
}

/** Compare complete file sets without traversing each byte as an object property. */
export async function expectFiles(
    actualBuild: PackageBuild,
    expectedBuild: PackageBuild,
): Promise<void> {
    const actual = await readBuildFiles(actualBuild);
    const expected = await readBuildFiles(expectedBuild);
    expect([...actual.keys()]).toEqual([...expected.keys()]);
    for (const [path, bytes] of actual) {
        expect(Buffer.compare(bytes, found(expected, path)), path).toBe(0);
    }
}

/** Read fixture outputs for complete byte comparisons. */
export async function readBuildFiles(
    build: PackageBuild,
): Promise<Map<string, Uint8Array<ArrayBuffer>>> {
    const files = new Map<string, Uint8Array<ArrayBuffer>>();
    const records = (await build.reader.distributed()).toSorted(comparePath);
    for (const file of records) {
        files.set(file.path, new Uint8Array(await readFile(join(build.directory, file.path))));
    }

    return files;
}

/** An isolated source checkout used by complete build fixtures. */
export class Fixture implements AsyncDisposable {
    /** Source fixture location. */
    readonly fixture: URL;
    /** Temporary fixture directory. */
    readonly directory: string;
    /** Copied package source directory. */
    readonly source: string;
    /** Resolved fixture dependencies. */
    readonly dependencies: BuildOptions["dependencies"];

    /** Reuse immutable dependency records across cases for the same fixture. */
    static readonly #dependencies = new Map<string, ReturnType<typeof readDependencies>>();

    /** Retain copied source and its immutable dependencies. */
    private constructor(
        fixture: URL,
        directory: string,
        source: string,
        dependencies: BuildOptions["dependencies"],
    ) {
        this.fixture = fixture;
        this.directory = directory;
        this.source = source;
        this.dependencies = dependencies;
    }

    /** Copy a fixture and resolve its installed dependencies. */
    static async open(name: string): Promise<Fixture> {
        const fixture = new URL(`./fixture/${name}/`, import.meta.url);

        // share dependency preparation across concurrent cases without caching edited source
        let dependencies = Fixture.#dependencies.get(name);
        if (!dependencies) {
            dependencies = readDependencies(fileURLToPath(new URL("source/", fixture)));
            Fixture.#dependencies.set(name, dependencies);
        }
        const resolved = await dependencies;

        // copy each case into a separate editable checkout
        const directory = await mkdtemp(join(tmpdir(), "destack-build-fixture-"));
        const source = join(directory, "source");
        try {
            await cp(new URL("source/", fixture), source, {
                recursive: true,
                filter: (path) =>
                    !path.endsWith("/node_modules") && !path.endsWith("\\node_modules"),
            });
            await linkDependencies(fileURLToPath(new URL("source/", fixture)), source);

            return new Fixture(fixture, directory, source, resolved);
        } catch (error) {
            await rm(directory, { recursive: true });
            throw error;
        }
    }

    /** Build the copied package with the fixture's output selection. */
    async build(request: Omit<BuildOptions, "directory" | "dependencies">): Promise<PackageBuild> {
        return await buildPackage({
            directory: this.source,
            dependencies: this.dependencies,
            ...request,
        });
    }

    /** Remove only this fixture's temporary checkout and distribution. */
    async [Symbol.asyncDispose](): Promise<void> {
        await rm(this.directory, { recursive: true });
    }
}

/** Read each manifest list and graph file independently from a relocated package. */
export async function expectManifest(build: PackageBuild, destination: string): Promise<void> {
    // load each file independently without touching executable files
    const loaded: string[] = [];
    const reader = new BuildReader(build.manifest, async (path) => {
        loaded.push(path);

        return new Uint8Array(await readFile(join(destination, path)));
    });

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
            await readFile(join(build.directory, reference.path), "utf8"),
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

/** Report whether a module's default export answers requests. */
export function isHandlerModule(module: object): module is HandlerModule {
    const handler = "default" in module ? module.default : undefined;

    return (
        typeof handler === "object" &&
        handler !== null &&
        "fetch" in handler &&
        typeof handler.fetch === "function"
    );
}

/** Report whether a module exports a workload and a service. */
export function isServiceModule(module: object): module is ServiceModule {
    const workload = "web" in module ? module.web : undefined;
    const service = "service" in module ? module.service : undefined;

    return (
        typeof workload === "object" &&
        workload !== null &&
        "start" in workload &&
        typeof workload.start === "function" &&
        typeof service === "object" &&
        service !== null &&
        "router" in service
    );
}
