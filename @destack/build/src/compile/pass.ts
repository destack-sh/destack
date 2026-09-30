import {
    copyFile,
    mkdir,
    mkdtemp,
    readdir,
    realpath,
    rm,
    symlink,
    writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import type { Manifest, Plugin } from "vite";
import type { DependencyResolution } from "@destack/package";
import type { Compilation, Pass } from "@destack/package/build";
import type { BuildDescription } from "@destack/package/inspect";
import type { SourceMapReference } from "@destack/package/source";
import { modulePlugin } from "@destack/package/transform/vite";
import { BuildError } from "../error/index.ts";
import { linkDependencies, type PackageSource } from "../source/index.ts";
import type { DirectoryReference } from "../typescript/index.ts";
import { describeAssets, directoryPlugin } from "./asset.ts";
import { dependencyPlugin, type ModuleSource } from "./dependency.ts";
import { checkRuntime, type RuntimeCompiler } from "./runtime.ts";
import { mapSource, sourcePlugin } from "./source.ts";

/** One pass of an output kind compiling several outputs, and the build's parts it lends the kind. */
export class OutputPass implements Pass {
    /** The opened package source. */
    readonly project: PackageSource;
    /** The dependency releases the build resolves against. */
    readonly dependencies: Readonly<Record<string, DependencyResolution>>;
    /** The inspected source bytes, by package path. */
    readonly sources: ReadonlyMap<string, Uint8Array<ArrayBuffer>>;
    /** The build directory. */
    readonly destination: string;
    /** The outputs the pass compiles, by name. */
    readonly compilations: Readonly<Record<string, Compilation>>;
    /** The directories the outputs' modules read, by module. */
    readonly directories: ReadonlyMap<string, readonly DirectoryReference[]>;
    /** The runtime environments checking each output's globals. */
    readonly runtimes: RuntimeCompiler;
    /** The generated files, by package path. */
    readonly files = new Map<string, Uint8Array<ArrayBuffer>>();
    /** The files written straight into the build directory. */
    readonly paths: string[] = [];
    /** The source maps of generated files. */
    readonly sourceMaps: SourceMapReference[] = [];
    /** What each output's modules import and emit, by output name. */
    readonly #inspections = new Map<string, BuildDescription>();
    /** The outputs a Vite environment records into. */
    readonly #recorded = new Set<string>();
    /** The authored location of each compiled module. */
    readonly #locations = new Map<string, ModuleSource>();
    /** The assets read beside modules, by path. */
    readonly #assets = new Map<string, Uint8Array<ArrayBuffer>>();

    /** Create one pass from the build's parts. */
    constructor(
        project: PackageSource,
        dependencies: Readonly<Record<string, DependencyResolution>>,
        sources: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
        destination: string,
        compilations: Readonly<
            Record<
                string,
                Compilation & {
                    readonly directories: ReadonlyMap<string, readonly DirectoryReference[]>;
                }
            >
        >,
        runtimes: RuntimeCompiler,
    ) {
        // keep the build's parts and one inspection per output
        this.project = project;
        this.dependencies = dependencies;
        this.sources = sources;
        this.destination = destination;
        this.compilations = compilations;
        this.directories = new Map(
            Object.values(compilations).flatMap((compilation) => [...compilation.directories]),
        );
        this.runtimes = runtimes;
        for (const name of Object.keys(compilations)) {
            this.#inspections.set(name, { packages: {}, inputs: {}, outputs: {} });
        }
    }

    /** Return the plugins retaining sources, directories and module metadata for every output. */
    plugins(): Plugin[] {
        return [
            directoryPlugin(this.project.directory, this.files, this.directories, this.#assets),
            sourcePlugin(
                this.project.directory,
                this.files,
                this.sources,
                this.destination,
                this.paths,
            ),
            modulePlugin(),
        ];
    }

    /** Return the plugin recording what one Vite environment compiles, into an output when it has one. */
    record(environment: string, output?: string): Plugin {
        // require an output of the pass, or record only the environment's module locations
        const inspection =
            output === undefined
                ? { packages: {}, inputs: {}, outputs: {} }
                : this.#inspections.get(output);
        if (inspection === undefined) {
            throw new BuildError("BUILD_FAILED", `the pass compiles no output ${output}`);
        }
        if (output !== undefined) {
            this.#recorded.add(output);
        }

        return dependencyPlugin(
            this.project,
            this.dependencies,
            inspection,
            `output/${output ?? environment}`,
            environment,
            this.#locations,
            this.#assets,
        );
    }

    /** Return what an output's modules import and emit, refusing an output no environment recorded. */
    inspection(output: string): BuildDescription {
        const inspection = this.#inspections.get(output);
        if (inspection === undefined || !this.#recorded.has(output)) {
            throw new BuildError("BUILD_FAILED", `the pass recorded no output ${output}`);
        }

        return inspection;
    }

    /** Map a generated file's source map source to its package path. */
    mapSource(source: string, map: string, generated: string): string {
        return mapSource(source, map, generated, this.#locations);
    }

    /** Describe a Vite asset manifest by package paths instead of paths below a root. */
    describeAssets(manifest: unknown, root: string): Manifest {
        return describeAssets(manifest as Manifest, root, this.#locations);
    }

    /** Check the modules of an output against its runtime, all of them unless paths select some. */
    async check(output: string, paths?: readonly string[]): Promise<void> {
        // require an output of the pass
        const compilation = this.compilations[output];
        if (compilation === undefined) {
            throw new BuildError("BUILD_FAILED", `the pass compiles no output ${output}`);
        }
        const inspection = this.inspection(output);

        // check the selected modules against the output's runtime
        const modules =
            paths === undefined
                ? compilation.modules
                : compilation.modules.filter((module) => paths.includes(module.path));
        await checkRuntime(inspection, modules, compilation.runtime, this.runtimes);
    }

    /** Stage the package in a temporary directory that resolves its dependencies. */
    async stage(): Promise<AsyncDisposable & { readonly directory: string }> {
        const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-build-")));

        // link package inputs while reserving the framework's output directory
        try {
            for (const entry of await readdir(this.project.directory, { withFileTypes: true })) {
                if (["dist", "node_modules", ".git", "tsconfig.json"].includes(entry.name)) {
                    continue;
                }
                await symlink(
                    join(this.project.directory, entry.name),
                    join(directory, entry.name),
                    entry.isDirectory() ? "junction" : "file",
                );
            }

            // resolve dependencies through the nearest installed package tree
            await linkDependencies(this.project.directory, directory);

            // retain authored relative paths through the selected absolute compiler configuration
            await writeFile(
                join(directory, "tsconfig.json"),
                JSON.stringify({ extends: this.project.configuration }),
            );
        } catch (error) {
            await rm(directory, { recursive: true });
            throw error;
        }

        return {
            directory,
            async [Symbol.asyncDispose]() {
                await rm(directory, { recursive: true });
            },
        };
    }

    /** Keep a generated file in the build. */
    file(path: string, bytes: Uint8Array<ArrayBuffer>): void {
        this.files.set(path, bytes);
    }

    /** Copy a generated file into the build. */
    async copy(path: string, source: string): Promise<void> {
        // copy the file and record its path
        const destination = join(this.destination, path);
        await mkdir(dirname(destination), { recursive: true });
        await copyFile(source, destination);
        this.paths.push(path);
    }

    /** Keep a generated file's source map. */
    sourceMap(generated: string, map: string): void {
        this.sourceMaps.push({ generated, map });
    }
}
