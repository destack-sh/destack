import { type DependencyResolution, PackageDefinition } from "@destack/package";
import type { History } from "@destack/resource";
import { mapExports, readPackageDescription } from "../source/source.ts";
import type { OutputRequest } from "@destack/package/build";
import { type ModuleOptions } from "../compile/compilation.ts";
import { mkdir, readFile, writeFile, copyFile, rm } from "node:fs/promises";
import { createReadStream } from "node:fs";
import { createHash } from "node:crypto";
import { dirname, resolve } from "node:path";
import { PackageManifest, BuildReader, type PackageDistribution } from "@destack/package/manifest";
import { PackagePath } from "@destack/package/file";
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
    /** Immutable inventory paths used by streamed reads. */
    #paths?: Promise<Set<string>>;

    /** Associate compiler output or imported files with their manifest. */
    constructor(manifest: PackageManifest, directory: string, ownership: "temporary" | "retained") {
        // retain the manifest, directory and reader
        this.manifest = manifest;
        this.directory = directory;
        this.ownership = ownership;
        this.reader = new BuildReader(
            manifest,
            async (path) =>
                new Uint8Array(await readFile(resolve(directory, PackagePath.parse(path)))),
        );
    }

    /** Stream a file selected from this build's inventory. */
    async open(path: string, signal?: AbortSignal): Promise<ReadableStream<Uint8Array>> {
        // require a path from the build inventory
        PackagePath.parse(path);
        this.#paths ??= this.reader
            .inventory()
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
                } else {
                    controller.enqueue(next.value as Uint8Array);
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
        const manifest = PackageManifest.parse(
            JSON.parse(await readFile(resolve(directory, "manifest.json"), "utf8")),
        );

        return new BuildReader(
            manifest,
            async (path) =>
                new Uint8Array(await readFile(resolve(directory, PackagePath.parse(path)))),
        );
    }

    /** Read a distributed build and verify its files. */
    static async read(directory: string): Promise<PackageBuild> {
        // read the manifest
        const manifest = PackageManifest.parse(
            JSON.parse(await readFile(resolve(directory, "manifest.json"), "utf8")),
        );

        // follow verified file references without retaining source or executable contents
        const reader = new BuildReader(manifest, async (path) => {
            return new Uint8Array(await readFile(resolve(directory, PackagePath.parse(path))));
        });
        const inventory = await reader.inventory();

        // reject ambiguous paths before opening source and executable files
        const paths = new Set<string>();
        for (const file of inventory) {
            if (file.path === "manifest.json" || file.path.startsWith("manifest.json/")) {
                throw new PackageError("INVALID_FILE", `reserved build path: ${file.path}`);
            }
            if (paths.has(file.path)) {
                throw new PackageError("INVALID_FILE", `duplicate file: ${file.path}`);
            }
            paths.add(file.path);
        }
        for (const path of paths) {
            for (
                let separator = path.indexOf("/");
                separator !== -1;
                separator = path.indexOf("/", separator + 1)
            ) {
                const parent = path.slice(0, separator);
                if (paths.has(parent)) {
                    throw new PackageError("INVALID_FILE", `file is also a directory: ${parent}`);
                }
            }
        }

        // keep deployed outputs separate and resolve their public files
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
        for (const directory of directories) {
            const segments = directory.split("/");
            for (let count = 1; count <= segments.length; count++) {
                const path = segments.slice(0, count).join("/");
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
        const references = Object.values(manifest.descriptions).map(
            (collection) => collection.file.path,
        );
        for (const output of Object.values(manifest.outputs)) {
            for (const path of Object.values(output.exports)) {
                if (!path.startsWith(`${output.directory}/`)) {
                    throw new PackageError("INVALID_FILE", `export is outside its output: ${path}`);
                }
                references.push(path);
            }
            for (const domain of Object.keys(output.descriptions)) {
                if (!manifest.descriptions[domain]) {
                    throw new PackageError(
                        "INVALID_FILE",
                        `unknown description collection: ${domain}`,
                    );
                }
            }
            if (output.tests.length && !manifest.tests) {
                throw new PackageError("INVALID_FILE", "output selects absent test declarations");
            }
        }
        for (const source of await reader.sourceMaps()) {
            references.push(source.generated, source.map);
        }
        for (const path of references) {
            if (!paths.has(path)) {
                throw new PackageError("INVALID_FILE", `missing build file: ${path}`);
            }
        }

        // verify distributed bytes without retaining complete files in memory
        for (const file of inventory) {
            const hash = createHash("sha256");
            let size = 0;
            for await (const bytes of createReadStream(resolve(directory, file.path))) {
                hash.update(bytes);
                size += bytes.length;
            }
            if (size !== file.size || hash.digest("hex") !== file.digest) {
                throw new PackageError("INVALID_FILE", `file contents differ: ${file.path}`);
            }
        }

        return new PackageBuild(manifest, directory, "retained");
    }

    /** Write a build to a new directory and publish its manifest last. */
    async write(directory: string): Promise<void> {
        // require a new directory so a failed write cannot damage an existing build
        await mkdir(directory);
        for (const file of await this.reader.inventory()) {
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

/** Inputs to a package build. */
export interface BuildOptions {
    /** The source package directory, kept unchanged for the duration of this build. */
    directory: string;
    /** Named module outputs, and outputs of kinds the package's dependencies compile. */
    outputs: Readonly<Record<string, ModuleOptions | OutputRequest>>;
    /** Exact dependency releases selected by the package resolver. */
    dependencies: Readonly<Record<string, DependencyResolution>>;
    /** The TypeScript configuration; omit to use package defaults. */
    configuration?: string;
    /** Cancel compilation and terminate its subprocesses. */
    signal?: AbortSignal;
    /** Maximum compilation time in milliseconds. */
    timeout?: number;
    /** What the package has published, to plan the upgrade from. */
    history?: History;
}

/** Read the outputs a package's exports imply: a browser module and one server module per server runtime. */
export async function readOutputs(directory: string): Promise<BuildOptions["outputs"]> {
    // read the package's declaration
    const declaration = await readPackageDescription(directory);
    const definition = declaration.definition;
    const outputs: Record<string, ModuleOptions> = {};
    for (const name of Object.keys(mapExports(declaration))) {
        // compile one module per runtime the export compiles for
        for (const runtime of PackageDefinition.runtimes(definition, name)) {
            outputs[runtime] = { kind: "module", runtime, bundle: true };
        }
    }

    return outputs;
}
