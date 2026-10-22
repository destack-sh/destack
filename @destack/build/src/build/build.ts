import { BuildError } from "../error/index.ts";
import {
    type DependencyResolution,
    PackageDefinition,
    type PackageJson,
    TEST_EXPORT,
} from "@destack/package";
import type { History } from "@destack/resource";
import type { LocalPackageStore } from "../store/local.ts";
import { mapExports } from "../source/source.ts";
import type { OutputRequest } from "@destack/package/build";
import { Runtime } from "@destack/package/runtime";
import { type Commit, schema, type Version } from "@destack/schema";
import { mkdir, writeFile, rm } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { readBuild } from "@destack/package/bun";
import { type BuildReader, MANIFEST_PATH, type PackageManifest } from "@destack/package/manifest";
import type { PackageFile } from "@destack/package/file";
import { digestFile } from "./directory.ts";
import { PackageError } from "@destack/package/error";

/** A build in a directory: its manifest, its files and their reader. */
export class PackageBuild implements AsyncDisposable {
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

    /** Keep a build directory's reader, its ownership and the outputs it reused. */
    constructor(
        reader: BuildReader,
        directory: string,
        ownership: "temporary" | "retained",
        reused: readonly string[] = [],
    ) {
        // keep the reader with its manifest, the directory and how the build owns it
        this.reader = reader;
        this.manifest = reader.manifest;
        this.directory = directory;
        this.ownership = ownership;
        this.reused = reused;
    }

    /** Remove only temporary results owned by this instance. */
    async [Symbol.asyncDispose](): Promise<void> {
        if (this.ownership === "temporary") {
            await rm(this.directory, { recursive: true, force: true });
        }
    }

    /** Write a stored build's files and manifest into a directory, reusing every output. */
    static async restore(
        store: LocalPackageStore,
        digest: string,
        destination: string,
    ): Promise<PackageBuild> {
        // clone the files, then write the manifest listing them
        const reader = await store.contents(digest);
        const { manifest } = reader;
        for (const file of await reader.distributed()) {
            await store.place(file, join(destination, file.path));
        }
        await writeFile(join(destination, MANIFEST_PATH), JSON.stringify(manifest), {
            flag: "wx",
        });

        // read the written build back, reusing every output
        const reused = Object.keys(manifest.outputs);

        return new PackageBuild(await readBuild(destination), destination, "temporary", reused);
    }

    /** Read a distributed build and verify its files. */
    static async read(directory: string): Promise<PackageBuild> {
        // follow verified file references without retaining source or executable contents
        const reader = await readBuild(directory);
        const { manifest } = reader;
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

        return new PackageBuild(reader, directory, "retained");
    }

    /** Write a build to a new directory, its manifest last, for the caller to move into place. */
    static async write(reader: BuildReader, directory: string): Promise<void> {
        // require a new directory so a failed write cannot damage an existing build
        await mkdir(directory);
        for (const file of await reader.distributed()) {
            const destination = resolve(directory, file.path);
            await mkdir(dirname(destination), { recursive: true });
            await writeFile(destination, await reader.load(file.path));
        }

        // make the build readable only after every file has been written
        await writeFile(resolve(directory, MANIFEST_PATH), JSON.stringify(reader.manifest), {
            flag: "wx",
        });
    }
}

/** Refuse the manifest path, a duplicate path or a file below another, returning the paths. */
function requireDistinctPaths(list: readonly PackageFile[]): Set<string> {
    // refuse reserved and duplicate paths
    const paths = new Set<string>();
    for (const file of list) {
        if (file.path === MANIFEST_PATH || file.path.startsWith(`${MANIFEST_PATH}/`)) {
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

/** List an output's exported files, refusing one outside its directory. */
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
    /** Minify generated code with Vite's minifier. */
    minify: schema.boolean().exactOptional(),
    /** Public URL prefix used by browser assets. */
    base: schema.string().exactOptional(),
});
/** Compilation settings for a named package output. */
export type ModuleOptions = schema.Infer<typeof ModuleOptions>;

/** Inputs to a package build. */
export interface PackageBuildOptions {
    /** The source package directory, kept unchanged for the duration of this build. */
    directory: string;
    /** Named module outputs, and outputs of kinds the dependency closure's extensions compile. */
    outputs: Readonly<Record<string, ModuleOptions | OutputRequest>>;
    /** Exact dependency releases selected by the package resolver. */
    dependencies: Readonly<Record<string, DependencyResolution>>;
    /** The TypeScript configuration, absent for the package defaults. */
    configuration?: string;
    /** Cancel compilation and terminate its subprocesses. */
    signal?: AbortSignal;
    /** The longest a compilation may take, in milliseconds, absent for no deadline. */
    timeout?: number;
    /** What the package has published, to plan its upgrade from. */
    history?: History;
    /** The commit of the source directory, absent for a working tree with changes. */
    commit?: Commit;
    /** The versions the build releases its source packages at, by package name. */
    versions?: Readonly<Record<string, Version>>;
}

/** List the module outputs a package ships, or the ones a build selects. */
export function outputsOf(
    packageJson: PackageJson,
    names?: readonly string[],
): Readonly<Record<string, ModuleOptions>> {
    // read the runtimes every export but the test layer compiles for
    const runtimes = new Set(
        Object.keys(mapExports(packageJson))
            .filter((name) => name !== TEST_EXPORT)
            .flatMap((name) => PackageDefinition.runtimes(packageJson.definition, name)),
    );

    // compile each as one module named after it, in the runtimes' declared order
    const implied = Object.fromEntries(
        Runtime.options
            .filter((runtime) => runtimes.has(runtime))
            .map((runtime) => [runtime, { kind: "module" as const, runtime }]),
    );
    if (names === undefined) {
        return implied;
    }

    // select the named outputs, refusing a name the exports imply no output for
    const unknown = names.filter((name) => !Object.hasOwn(implied, name));
    if (unknown.length > 0) {
        throw new BuildError("BUILD_FAILED", `the package builds no output ${unknown.join(", ")}`);
    }

    return Object.fromEntries(Object.entries(implied).filter(([name]) => names.includes(name)));
}
