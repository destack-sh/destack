import { type DependencyResolution, PackageDefinition } from "@destack/package";
import type { History } from "@destack/resource";
import type { PackageStore } from "../store/index.ts";
import { mapExports, readPackageDescription, TEST_EXPORT } from "../source/source.ts";
import type { OutputRequest } from "@destack/package/build";
import { Runtime } from "@destack/package/runtime";
import { type Commit, schema } from "@destack/schema";
import { mkdir, readFile, writeFile, copyFile, rm } from "node:fs/promises";
import { createReadStream } from "node:fs";
import { dirname, resolve } from "node:path";
import { BuildReader, PackageManifest, type PackageDistribution } from "@destack/package/manifest";
import { type PackageFile, PackagePath } from "@destack/package/file";
import { digestFile } from "./file.ts";
import { PackageError } from "@destack/package/error";

/** A package manifest and its file-backed distribution. */
export class PackageBuild implements PackageDistribution, AsyncDisposable {
    /** The package build manifest. */
    readonly manifest: PackageManifest;
    /** Directory containing the complete distribution. */
    readonly directory: string;
    /** Whether disposal removes the result directory. */
    readonly ownership: "temporary" | "retained";
    /** Selective access to the build's description files. */
    readonly reader: BuildReader;
    /** The requested outputs the build reused from a cache instead of compiling them. */
    readonly reused: readonly string[];
    /** Immutable list paths used by streamed reads. */
    #paths?: Promise<Set<string>>;

    /** Associate compiler output or imported files with their manifest. */
    constructor(
        manifest: PackageManifest,
        directory: string,
        ownership: "temporary" | "retained",
        reused: readonly string[] = [],
    ) {
        // retain the manifest and the directory with its reused outputs
        this.manifest = manifest;
        this.directory = directory;
        this.ownership = ownership;
        this.reused = reused;

        // open the reader of the build's files
        this.reader = openReader(manifest, directory);
    }

    /** Stream a file this build's file list names. */
    async open(path: string, signal?: AbortSignal): Promise<ReadableStream<Uint8Array>> {
        // require a path the file list names
        PackagePath.parse(path);
        this.#paths ??= this.reader
            .distributed()
            .then((files) => new Set(files.map((file) => file.path)));
        if (!(await this.#paths).has(path)) {
            throw new PackageError("INVALID_FILE", `unknown build file: ${path}`);
        }

        // pull the file's chunks as the web stream's reader asks for them
        const chunks = createReadStream(resolve(this.directory, path), { signal })[
            Symbol.asyncIterator
        ]();

        return new ReadableStream<Uint8Array>({
            pull: async (controller) => {
                const next = await chunks.next();
                if (next.done === true) {
                    controller.close();
                } else if (next.value instanceof Uint8Array) {
                    controller.enqueue(next.value);
                } else {
                    throw new TypeError(`file stream of ${path} yielded no bytes`);
                }
            },
            cancel: async () => {
                await chunks.return?.();
            },
        });
    }

    /** Remove only temporary results owned by this instance. */
    async [Symbol.asyncDispose](): Promise<void> {
        if (this.ownership === "temporary") {
            await rm(this.directory, { recursive: true, force: true });
        }
    }

    /** Read only the root manifest and load each description when requested. */
    static async open(directory: string): Promise<BuildReader> {
        return openReader(await readManifest(directory), directory);
    }

    /** Read a distributed build and verify its files. */
    static async read(directory: string): Promise<PackageBuild> {
        // follow verified file references without retaining source or executable contents
        const manifest = await readManifest(directory);
        const reader = openReader(manifest, directory);
        const list = await reader.distributed();

        // reject ambiguous paths and outputs before opening source and executable files
        const paths = requireDistinctPaths(list);
        requireSeparateOutputs(manifest, paths);
        const references = Object.values(manifest.outputs).flatMap((output) =>
            exportedPaths(manifest, output),
        );
        for (const source of await reader.sourceMaps()) {
            references.push(source.generated, source.map);
        }
        for (const path of references) {
            if (!paths.has(path)) {
                throw new PackageError("INVALID_FILE", `missing build file: ${path}`);
            }
        }

        // verify distributed bytes without retaining complete files in memory
        for (const file of list) {
            const { digest, size } = await digestFile(resolve(directory, file.path));
            if (size !== file.size || digest !== file.digest) {
                throw new PackageError("INVALID_FILE", `file contents differ: ${file.path}`);
            }
        }

        return new PackageBuild(manifest, directory, "retained");
    }

    /** Write a build to a new directory and publish its manifest last. */
    async write(directory: string): Promise<void> {
        // require a new directory so a failed write cannot damage an existing build
        await mkdir(directory);
        for (const file of await this.reader.distributed()) {
            const destination = resolve(directory, file.path);
            await mkdir(dirname(destination), { recursive: true });
            await copyFile(resolve(this.directory, file.path), destination);
        }

        // make the build readable only after every file has been written
        await writeFile(resolve(directory, "manifest.json"), JSON.stringify(this.manifest), {
            flag: "wx",
        });
    }
}

/** Read a build directory's manifest. */
async function readManifest(directory: string): Promise<PackageManifest> {
    return PackageManifest.parse(
        JSON.parse(await readFile(resolve(directory, "manifest.json"), "utf8")),
    );
}

/** Read a build's description files from its directory. */
function openReader(manifest: PackageManifest, directory: string): BuildReader {
    return new BuildReader(
        manifest,
        async (path) => new Uint8Array(await readFile(resolve(directory, PackagePath.parse(path)))),
    );
}

/** Refuse a file list naming the reserved manifest path, one path twice or a file below another, returning the paths. */
function requireDistinctPaths(list: readonly PackageFile[]): Set<string> {
    // refuse reserved and duplicate paths
    const paths = new Set<string>();
    for (const file of list) {
        if (file.path === "manifest.json" || file.path.startsWith("manifest.json/")) {
            throw new PackageError("INVALID_FILE", `reserved build path: ${file.path}`);
        }
        if (paths.has(file.path)) {
            throw new PackageError("INVALID_FILE", `duplicate file: ${file.path}`);
        }
        paths.add(file.path);
    }

    // refuse a file whose parent directory is also a file
    for (const path of paths) {
        for (const parent of parents(path)) {
            if (paths.has(parent)) {
                throw new PackageError("INVALID_FILE", `file is also a directory: ${parent}`);
            }
        }
    }

    return paths;
}

/** Refuse outputs sharing or nesting directories, or a directory that is a file. */
function requireSeparateOutputs(manifest: PackageManifest, paths: ReadonlySet<string>): void {
    // refuse two outputs in one directory
    const directories = new Set<string>();
    for (const output of Object.values(manifest.outputs)) {
        if (directories.has(output.directory)) {
            throw new PackageError(
                "INVALID_FILE",
                `duplicate output directory: ${output.directory}`,
            );
        }
        directories.add(output.directory);
    }

    // refuse an output directory below a file or below another output's directory
    for (const directory of directories) {
        for (const path of [...parents(directory), directory]) {
            if (paths.has(path)) {
                throw new PackageError("INVALID_FILE", `output directory is a file: ${path}`);
            }
            if (path !== directory && directories.has(path)) {
                throw new PackageError(
                    "INVALID_FILE",
                    `overlapping output directories: ${path}, ${directory}`,
                );
            }
        }
    }
}

/** List an output's exported files, refusing one outside its directory or tests the build lacks. */
function exportedPaths(
    manifest: PackageManifest,
    output: PackageManifest["outputs"][string],
): string[] {
    // refuse exports outside the output's directory
    const exported = Object.values(output.exports);
    for (const path of exported) {
        if (!path.startsWith(`${output.directory}/`)) {
            throw new PackageError("INVALID_FILE", `export is outside its output: ${path}`);
        }
    }

    // refuse selected tests without test declarations
    if (output.tests.length && !manifest.tests) {
        throw new PackageError("INVALID_FILE", "output selects absent test declarations");
    }

    return exported;
}

/** List the directories above a package path, outermost first. */
function parents(path: string): string[] {
    // join each leading run of segments
    const segments = path.split("/");
    const found: string[] = [];
    for (let count = 1; count < segments.length; count++) {
        found.push(segments.slice(0, count).join("/"));
    }

    return found;
}

/** Compilation settings for a named package output. */
export const ModuleOptions = schema.object({
    /** Compile package modules. */
    kind: schema.literal("module"),
    /** The runtime the output is compiled for. */
    runtime: Runtime,
    /** Package-relative module or HTML entries, absent to compile the package exports. */
    entries: schema.record(schema.string(), schema.string()).readonly().exactOptional(),
    /** Bundle dependencies, off for library dependencies installed separately. */
    bundle: schema.boolean().exactOptional(),
    /** Minify generated code with Vite's minifier. */
    minify: schema.boolean().exactOptional(),
    /** Public URL prefix used by browser assets. */
    base: schema.string().exactOptional(),
});
/** Compilation settings for a named package output. */
export type ModuleOptions = schema.Infer<typeof ModuleOptions>;

/** Inputs to a package build. */
export interface BuildOptions {
    /** The source package directory, kept unchanged for the duration of this build. */
    directory: string;
    /** Named module outputs, and outputs of kinds the extensions of the package's dependency closure compile. */
    outputs: Readonly<Record<string, ModuleOptions | OutputRequest>>;
    /** Exact dependency releases selected by the package resolver. */
    dependencies: Readonly<Record<string, DependencyResolution>>;
    /** The TypeScript configuration, absent for the package defaults. */
    configuration?: string;
    /** Cancel compilation and terminate its subprocesses. */
    signal?: AbortSignal;
    /** Maximum compilation time in milliseconds. */
    timeout?: number;
    /** What the package has published, to plan the upgrade from. */
    history?: History;
    /** The commit the source directory holds, absent for a working tree with uncommitted changes. */
    commit?: Commit;
    /** The store whose cache the build reuses and fills, absent to build without a cache. */
    store?: PackageStore;
}

/** Read the outputs a package's exports imply: a browser module and one server module per server runtime. */
export async function readOutputs(
    directory: string,
): Promise<Readonly<Record<string, ModuleOptions>>> {
    // read the package's declaration
    const declaration = await readPackageDescription(directory);
    const definition = declaration.definition;
    const outputs: Record<string, ModuleOptions> = {};
    for (const name of Object.keys(mapExports(declaration))) {
        // compile one module per runtime each shipped export compiles for
        if (name === TEST_EXPORT) {
            continue;
        }
        for (const runtime of PackageDefinition.runtimes(definition, name)) {
            outputs[runtime] = { kind: "module", runtime, bundle: true };
        }
    }

    return outputs;
}
