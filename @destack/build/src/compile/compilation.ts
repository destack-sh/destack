import { readFile } from "node:fs/promises";
import { isAbsolute, join, relative, resolve, sep } from "node:path";
import { build, type InlineConfig, type Plugin, type PluginOption } from "vite";
import type {
    LogLevel,
    LogOrStringHandler,
    OutputAsset,
    OutputChunk,
    RolldownOptions,
    RolldownLog,
} from "rolldown";
import {
    DependencyName,
    DependencyRelease,
    type DependencyResolution,
    type Package,
} from "@destack/package";
import {
    type BuildExtension,
    type Compilation,
    type CompiledOutput,
    type DeclarationModule,
    isHostModule,
    isRuntimeModule,
    type OutputDescription,
    runtimeConditions,
} from "@destack/package/build";
import type { ModuleDescription } from "@destack/package/code";
import type { Capabilities } from "@destack/package";
import { PackagePath } from "@destack/package/file";
import type { BuildDescription, DeclarationDescription } from "@destack/package/inspect";
import { type PackageOutput } from "@destack/package/manifest";
import { type SourceMapReference } from "@destack/package/source";
import { modulePlugin } from "@destack/package/transform/vite";
import { type Runtime } from "@destack/package/runtime";
import { type PackageSource } from "../source/index.ts";
import { relativePath } from "../source/dependency.ts";
import { type DirectoryReference } from "../typescript/index.ts";
import { type ModuleOptions } from "../build/build.ts";
import { BuildError } from "../error/index.ts";
import { sourcePlugin, mapSource } from "./source.ts";
import { dependencyPlugin, modulePath, type ModuleSource } from "./dependency.ts";
import { directoryPlugin } from "./asset.ts";

/** The files and descriptions one compiled output adds to its build. */
export interface CompiledFiles {
    /** Dependencies, module imports, and generated file membership. */
    inspection: BuildDescription;
    /** The target output description. */
    output: PackageOutput;
    /** Generated files and captured source assets. */
    files: Map<string, Uint8Array<ArrayBuffer>>;
    /** Generated output paths written by Vite. */
    paths: string[];
    /** Maps for generated JavaScript. */
    sourceMaps: SourceMapReference[];
}

/** The resolved dependencies, sources, destination and entries one compilation reads. */
interface CompileInput {
    /** The resolved dependencies, by name. */
    readonly dependencies: Readonly<Record<string, DependencyResolution>>;
    /** The source bytes the build snapshots, by absolute path. */
    readonly sources: ReadonlyMap<string, Uint8Array<ArrayBuffer>>;
    /** The directory the build writes under. */
    readonly destinationRoot: string;
    /** The absolute source modules the output compiles. */
    readonly entries: readonly string[];
}

/** One module output a build compiles with the extensions of the package's dependencies. */
export class OutputCompilation implements Compilation {
    /** The output's name. */
    readonly name: string;
    /** The output's settings. */
    readonly options: ModuleOptions;
    /** The opened package source. */
    readonly project: PackageSource;
    /** The declarations of the package and of the dependencies its modules import. */
    readonly declarations: readonly DeclarationDescription[];
    /** The package's inspected modules. */
    readonly modules: readonly ModuleDescription[];
    /** The directories the package's modules read, by module. */
    readonly directories: ReadonlyMap<string, readonly DirectoryReference[]>;
    /** The extensions of the package's dependencies. */
    readonly extensions: readonly BuildExtension[];
    /** Whether an output kind compiles the output in its pass, where extensions emit no entries. */
    readonly isLent: boolean;
    /** The modules extensions emit, by package entrypoint. */
    readonly #entries = new Map<string, string>();

    /** Create the compilation of one output from its settings, package and inspection. */
    constructor(
        name: string,
        options: ModuleOptions,
        project: PackageSource,
        inspection: {
            readonly declarations: readonly DeclarationDescription[];
            readonly modules: readonly ModuleDescription[];
            readonly directories: ReadonlyMap<string, readonly DirectoryReference[]>;
        },
        extensions: readonly BuildExtension[],
        isLent: boolean,
    ) {
        // keep the output's settings, package and inspection
        this.name = name;
        this.options = options;
        this.project = project;
        this.declarations = inspection.declarations;
        this.modules = inspection.modules;
        this.directories = inspection.directories;
        this.extensions = extensions;
        this.isLent = isLent;
    }

    /** The package the build compiles. */
    get package(): Package {
        return this.project.declaration.package;
    }

    /** The package's source directory. */
    get directory(): string {
        return this.project.directory;
    }

    /** The absolute source modules the output compiles, by export path. */
    get exports(): Readonly<Record<string, string>> {
        return this.project.exports;
    }

    /** The runtime the output runs on. */
    get runtime(): Runtime {
        return this.project.runtime;
    }

    /** What the package's workloads and views may access. */
    get capabilities(): Capabilities {
        return this.project.declaration.definition.capabilities ?? {};
    }

    /** Locate the module and export of one of the package's declarations. */
    locate(declaration: DeclarationDescription): DeclarationModule {
        // match the declaring module's export resolving to the declared constant
        const { module, name } = declaration.symbol.symbol;
        const exported = this.modules
            .find((entry) => entry.path === module)
            ?.exports.find(
                (entry) =>
                    !entry.isTypeOnly &&
                    "module" in entry.symbol &&
                    entry.symbol.module === module &&
                    entry.symbol.name === name,
            );
        if (exported === undefined) {
            throw new BuildError(
                "BUILD_FAILED",
                `${declaration.kind} ${declaration.name} has no export`,
            );
        }

        return { file: join(this.project.directory, module), export: exported.name };
    }

    /** Emit a module as a chunk the package exports under an entrypoint. */
    entry(entrypoint: string, module: string): void {
        // refuse an entry of an output its kind compiles in a pass
        if (this.isLent) {
            throw new BuildError(
                "BUILD_FAILED",
                `a pass compiles output ${this.name}, which emits no entry: ${entrypoint}`,
            );
        }

        // refuse an entrypoint taken twice
        if (this.#entries.has(entrypoint) || Object.hasOwn(this.project.exports, entrypoint)) {
            throw new BuildError("BUILD_FAILED", `duplicate entrypoint: ${entrypoint}`);
        }
        this.#entries.set(entrypoint, module);
    }

    /** Compile the output with Vite and the extensions' plugins, then let the extensions describe it. */
    async compile(
        dependencies: Readonly<Record<string, DependencyResolution>>,
        sources: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
        destinationRoot: string,
    ): Promise<CompiledFiles> {
        // collect the extensions' plugins and entries before compiling
        const { project } = this;
        const compiled = this.#createFiles();
        const plugins = this.#plugins();
        const entries = Object.values(project.exports);
        if (!entries.length && !this.#entries.size) {
            return compiled;
        }
        if (entries.some((entry) => entry.endsWith(".html")) && project.runtime !== "browser") {
            throw new BuildError("BUILD_FAILED", "entries in HTML require the browser runtime");
        }

        // retain HTML before Vite rewrites its module and asset references
        await retainPages(project.directory, entries, compiled.files);

        // delegate JSX, CSS, assets, and minification to the extensions and standard plugins
        const facades = new Map<string, string>();
        const configuration = this.#configuration(compiled, facades, plugins, {
            dependencies,
            sources,
            destinationRoot,
            entries,
        });
        const generated = await runVite(configuration);

        // map authored and emitted entries to generated files
        this.#collect(generated, compiled, facades);
        this.#exportEntries(generated, compiled.output.exports, facades);

        // let each extension describe the compiled output's workloads and views
        this.#describeOutput(compiled, facades);

        return compiled;
    }

    /** Create the empty files and descriptions of the output under its output directory. */
    #createFiles(): CompiledFiles {
        const output: PackageOutput = {
            runtime: this.project.runtime,
            emit: true,
            workloads: {},
            views: {},
            tests: [],
            directory: `output/${this.name}`,
            exports: {},
            dependencies: {},
        };

        return {
            inspection: { packages: {}, inputs: {}, outputs: {} },
            output,
            files: new Map(),
            paths: [],
            sourceMaps: [],
        };
    }

    /** Let each extension describe the compiled output's workloads and views. */
    #describeOutput(compiled: CompiledFiles, facades: ReadonlyMap<string, string>): void {
        // expose the exports and the declarations of each entrypoint
        const { inspection, output } = compiled;
        const exports = output.exports;
        const described: CompiledOutput = {
            exports,
            declarations: (entrypoint) =>
                this.#declarationsOf(entrypoint, exports, facades, inspection),
        };

        // merge each extension's description into the output
        for (const extension of this.extensions) {
            merge(output, describe(extension, this, described));
        }
    }

    /** Collect the extensions' plugins, reporting an extension's failure as a build failure. */
    #plugins(): PluginOption[] {
        return this.extensions.flatMap((extension) => {
            try {
                return extension.compile?.(this) ?? [];
            } catch (cause) {
                throw BuildError.from(cause);
            }
        });
    }

    /** Configure Vite to compile the output's entries into the build directory. */
    #configuration(
        compiled: CompiledFiles,
        facades: Map<string, string>,
        plugins: readonly PluginOption[],
        input: CompileInput,
    ): InlineConfig {
        // read the output's settings and the runtime's resolution
        const { options, project } = this;
        const destination = resolve(input.destinationRoot, compiled.output.directory);
        const locations = new Map<string, ModuleSource>();
        const conditions = runtimeConditions(project.runtime);

        return {
            root: project.directory,
            configFile: false,
            envDir: false,
            publicDir: false,
            base: options.base ?? "./",
            logLevel: "silent",
            define: { "process.env.NODE_ENV": JSON.stringify("production") },
            resolve: { conditions },
            ssr: {
                noExternal: true,
                target: project.runtime === "workerd" ? "webworker" : "node",
                resolve: { conditions },
            },
            plugins: [
                ...this.#packagePlugins(compiled, facades, input, locations),
                ...plugins,
                modulePlugin(),
            ],
            build: {
                emitAssets: true,
                write: true,
                outDir: destination,
                emptyOutDir: false,
                ssr: project.runtime !== "browser",
                minify: options.minify ?? false,
                sourcemap: true,
                copyPublicDir: false,
                rolldownOptions: this.#rolldownOptions(compiled, input, destination, locations),
            },
        };
    }

    /** Create the plugins for the package's directories, sources, dependencies and entries. */
    #packagePlugins(
        compiled: CompiledFiles,
        facades: Map<string, string>,
        input: CompileInput,
        locations: Map<string, ModuleSource>,
    ): Plugin[] {
        // share captured assets between the directory and dependency plugins
        const { project } = this;
        const assets = new Map<string, Uint8Array<ArrayBuffer>>();

        return [
            directoryPlugin(project.directory, compiled.files, this.directories, assets),
            sourcePlugin(
                project.directory,
                compiled.files,
                input.sources,
                input.destinationRoot,
                compiled.paths,
            ),
            dependencyPlugin(
                project,
                input.dependencies,
                compiled.inspection,
                compiled.output.directory,
                undefined,
                locations,
                assets,
            ),
            this.#entryPlugin(facades),
        ];
    }

    /** Configure Rolldown's resolution, inputs, externals and generated file names. */
    #rolldownOptions(
        compiled: CompiledFiles,
        input: CompileInput,
        destination: string,
        locations: ReadonlyMap<string, ModuleSource>,
    ): RolldownOptions {
        // resolve for the runtime's platform
        const { project } = this;
        const directory = compiled.output.directory;
        const isBun = project.runtime === "bun";

        return {
            platform: isBun ? "node" : "browser",
            resolve: {
                mainFields: isBun ? ["module", "main"] : ["browser", "module", "main"],
                conditionNames: runtimeConditions(project.runtime),
            },
            cwd: project.directory,
            experimental: { attachDebugInfo: "none" },
            input: [...input.entries],
            preserveEntrySignatures: "strict",
            // ignore plugin timings
            checks: { pluginTimings: false },
            onLog: failOnWarning,
            external: (specifier, importer) =>
                this.#isExternal(specifier, importer, input.dependencies, compiled.output),
            output: {
                format: "esm",
                entryFileNames: "[name]-[hash].js",
                chunkFileNames: "[name]-[hash].js",
                assetFileNames: "asset/[name]-[hash][extname]",
                sourcemapPathTransform(source, map) {
                    const generatedMap = `${directory}/${relativePath(destination, map)}`;

                    return mapSource(source, map, generatedMap, locations);
                },
            },
        };
    }

    /** Retain generated paths and source maps, and map the package's exports to generated files. */
    #collect(
        generated: readonly (OutputChunk | OutputAsset)[],
        compiled: CompiledFiles,
        facades: Map<string, string>,
    ): void {
        const { project } = this;
        const directory = compiled.output.directory;
        for (const entry of generated) {
            // retain the generated path
            const path = PackagePath.parse(`${directory}/${entry.fileName}`);
            compiled.paths.push(path);

            // export a module entry's chunk and an HTML entry's page
            for (const [name, source] of Object.entries(project.exports)) {
                if (entry.type === "chunk" && entry.facadeModuleId === source) {
                    compiled.output.exports[name] = path;
                    facades.set(name, source);
                }
                if (
                    entry.type === "asset" &&
                    source.endsWith(".html") &&
                    entry.fileName === relativePath(project.directory, source)
                ) {
                    compiled.output.exports[name] = path;
                }
            }

            // keep a chunk's source map
            if (entry.type === "chunk" && entry.sourcemapFileName !== null) {
                compiled.sourceMaps.push({
                    generated: path,
                    map: `${directory}/${entry.sourcemapFileName}`,
                });
            }
        }
    }

    /** Export each emitted entry's chunk under its entrypoint, requiring every export to be generated. */
    #exportEntries(
        generated: readonly (OutputChunk | OutputAsset)[],
        exports: Record<string, string>,
        facades: ReadonlyMap<string, string>,
    ): void {
        // export each entry's emitted chunk under its entrypoint
        for (const entrypoint of this.#entries.keys()) {
            const facade = facades.get(entrypoint);
            const chunk = generated.find(
                (entry) => entry.type === "chunk" && entry.facadeModuleId === facade,
            );
            if (chunk === undefined) {
                throw new BuildError("BUILD_FAILED", `missing generated entry: ${entrypoint}`);
            }
            exports[entrypoint] = PackagePath.parse(`output/${this.name}/${chunk.fileName}`);
        }

        // require every package export to be generated
        for (const name of Object.keys(this.project.exports)) {
            if (exports[name] === undefined) {
                throw new BuildError("BUILD_FAILED", `missing generated entry: ${name}`);
            }
        }
    }

    /** Emit a chunk per entry and remember the module each chunk starts from. */
    #entryPlugin(facades: Map<string, string>): Plugin {
        const entries = this.#entries;
        const references = new Map<string, string>();

        return {
            name: "@destack/build/entry",
            buildStart() {
                // emit each entry as a chunk with its entrypoint as name
                for (const [entrypoint, module] of entries) {
                    const name = entrypoint.replace(/^\.\//u, "").replaceAll("/", "-");
                    const reference = this.emitFile({
                        type: "chunk",
                        id: module,
                        name,
                        preserveSignature: "strict",
                    });
                    references.set(entrypoint, reference);
                }
            },
            generateBundle(_, bundle) {
                // remember the module starting each entry's chunk
                for (const [entrypoint, reference] of references) {
                    const chunk = bundle[this.getFileName(reference)];
                    if (chunk?.type === "chunk" && chunk.facadeModuleId !== null) {
                        facades.set(entrypoint, chunk.facadeModuleId);
                    }
                }
            },
        };
    }

    /** Keep host modules and registry releases external, refusing undeclared dependencies. */
    #isExternal(
        specifier: string,
        importer: string | undefined,
        dependencies: Readonly<Record<string, DependencyResolution>>,
        output: PackageOutput,
    ): boolean {
        // keep host modules the runtime supplies, refusing the rest before Vite emits browser stubs
        const { project } = this;
        if (isHostModule(specifier)) {
            if (!isRuntimeModule(specifier, project.runtime)) {
                throw new BuildError(
                    "BUILD_FAILED",
                    `host module is unavailable on ${project.runtime}: ${specifier}`,
                );
            }

            return true;
        }

        // bundle local modules and what bundled dependencies import
        if (this.#isBundled(specifier, importer)) {
            return false;
        }

        // bundle a declared dependency, or keep its registry release external
        return this.#isExternalRelease(DependencyName.of(specifier), dependencies, output);
    }

    /** Check whether a specifier names a local module or one a bundled dependency imports. */
    #isBundled(specifier: string, importer: string | undefined): boolean {
        // bundle relative, absolute, virtual and package-internal modules
        const { options, project } = this;
        if (
            specifier.startsWith(".") ||
            specifier.startsWith("/") ||
            specifier.startsWith("\0") ||
            specifier.startsWith("#") ||
            DependencyName.of(specifier) === project.declaration.package.name
        ) {
            return true;
        }

        // bundle what bundled dependencies import
        return (
            options.bundle === true &&
            importer !== undefined &&
            (importer.split(sep).includes("node_modules") ||
                relative(project.directory, importer).startsWith(`..${sep}`))
        );
    }

    /** Require a declared dependency, keeping its registry release external unless bundled. */
    #isExternalRelease(
        dependency: string,
        dependencies: Readonly<Record<string, DependencyResolution>>,
        output: PackageOutput,
    ): boolean {
        // require a declared dependency
        const { declaration } = this.project;
        const groups = [
            declaration.dependencies,
            declaration.peerDependencies,
            declaration.optionalDependencies,
        ];
        if (!groups.some((requirements) => Object.hasOwn(requirements, dependency))) {
            throw new BuildError("BUILD_FAILED", `undeclared runtime dependency: ${dependency}`);
        }

        // bundle it, or keep its registry release external
        if (this.options.bundle === true) {
            return false;
        }
        const selected = dependencies[dependency];
        if (!selected) {
            throw new BuildError("BUILD_FAILED", `unresolved runtime dependency: ${dependency}`);
        }
        if (selected.kind === "source") {
            throw new BuildError(
                "BUILD_FAILED",
                `runtime dependency requires a registry release: ${dependency}`,
            );
        }
        output.dependencies[dependency] = DependencyRelease.parse(selected);

        return true;
    }

    /** Select the declarations an entrypoint's modules declare, including declarations shaken out of its chunks. */
    #declarationsOf(
        entrypoint: string,
        exports: Readonly<Record<string, string>>,
        facades: ReadonlyMap<string, string>,
        inspection: BuildDescription,
    ): DeclarationDescription[] {
        // require an emitted entrypoint whose chunks import only what the runtime supplies
        const emitted = exports[entrypoint];
        const facade = facades.get(entrypoint);
        if (emitted === undefined || facade === undefined) {
            throw new BuildError("BUILD_FAILED", `no emitted entrypoint: ${entrypoint}`);
        }
        this.#checkChunks(entrypoint, emitted, inspection);

        // start from the entry's module as the dependency plugin records it
        const root = isAbsolute(facade)
            ? relativePath(this.directory, facade)
            : facade.replaceAll(this.directory, ".");
        const sources = Object.entries(inspection.inputs)
            .filter(([, input]) => input.package === undefined && modulePath(input.path) === root)
            .map(([id]) => id);
        if (!sources.length) {
            throw new BuildError("BUILD_FAILED", `missing input of entrypoint: ${entrypoint}`);
        }

        // select the declarations the files of the entry's modules declare
        const paths = this.#readFiles(sources, inspection);

        return this.declarations.filter((declaration) => {
            const owner = declaration.symbol.package;

            return (
                paths.get(`${owner.name}@${owner.version}`)?.has(declaration.source.file) === true
            );
        });
    }

    /** Refuse external imports of an entry's chunks that the runtime lacks. */
    #checkChunks(entrypoint: string, emitted: string, inspection: BuildDescription): void {
        const chunks = [emitted];
        const visited = new Set<string>();
        for (const path of chunks) {
            // visit each chunk once
            if (visited.has(path)) {
                continue;
            }
            visited.add(path);
            const chunk = inspection.outputs[path];
            if (!chunk) {
                throw new BuildError("BUILD_FAILED", `unknown emitted chunk: ${path}`);
            }

            // follow internal imports and check external ones against the runtime
            for (const imported of chunk.imports) {
                if (!imported.external) {
                    chunks.push(imported.path);
                } else if (!isRuntimeModule(imported.path, this.runtime)) {
                    throw new BuildError(
                        "BUILD_FAILED",
                        `unsupported external import of ${entrypoint} on ${this.runtime}: ${imported.path}`,
                    );
                }
            }
        }
    }

    /** Index the source files each package's modules read, following imports from some inputs. */
    #readFiles(sources: string[], inspection: BuildDescription): Map<string, Set<string>> {
        // walk the inputs breadth first
        const visited = new Set<string>();
        const paths = new Map<string, Set<string>>();
        for (const id of sources) {
            // visit each input once
            if (visited.has(id)) {
                continue;
            }
            visited.add(id);
            const input = inspection.inputs[id];
            if (!input) {
                throw new BuildError("BUILD_FAILED", `unknown source module: ${id}`);
            }

            // index the input's file under its package
            const owner =
                input.package === undefined
                    ? this.package
                    : inspection.packages[input.package]?.package;
            if (owner === undefined) {
                throw new BuildError("BUILD_FAILED", `unknown source package: ${input.package}`);
            }
            const key = `${owner.name}@${owner.version}`;
            const files = paths.get(key) ?? new Set<string>();
            files.add(modulePath(input.path));
            paths.set(key, files);

            // follow the input's internal imports
            for (const imported of input.imports) {
                if (!imported.external) {
                    sources.push(imported.path);
                }
            }
        }

        return paths;
    }
}

/** Retain each HTML entry's authored bytes under its package path. */
async function retainPages(
    directory: string,
    entries: readonly string[],
    files: Map<string, Uint8Array<ArrayBuffer>>,
): Promise<void> {
    for (const entry of entries.filter((path) => path.endsWith(".html"))) {
        files.set(relativePath(directory, entry), new Uint8Array(await readFile(entry)));
    }
}

/** Run Vite and return the chunks and assets of its one output. */
async function runVite(configuration: InlineConfig): Promise<(OutputChunk | OutputAsset)[]> {
    // report a bundler failure as a build failure
    const generated = await build(configuration).catch((error: unknown) => {
        throw BuildError.fromBundle(error);
    });
    if (!("output" in generated)) {
        throw new BuildError("BUILD_FAILED", "expected one Vite output");
    }

    return generated.output;
}

/** Fail on Rolldown warnings and pass other logs on. */
function failOnWarning(level: LogLevel, log: RolldownLog, handler: LogOrStringHandler): void {
    if (level === "warn") {
        throw new BuildError("BUILD_FAILED", log.message);
    }
    handler(level, log);
}

/** Ask an extension to describe a compiled output, reporting its failure as a build failure. */
function describe(
    extension: BuildExtension,
    compilation: Compilation,
    compiled: CompiledOutput,
): OutputDescription {
    try {
        return extension.describe?.(compilation, compiled) ?? {};
    } catch (cause) {
        throw BuildError.from(cause);
    }
}

/** Add an extension's workloads and views to an output, refusing a name described twice. */
function merge(output: PackageOutput, described: OutputDescription): void {
    mergeSection("workloads", output.workloads, described.workloads);
    mergeSection("views", output.views, described.views);
}

/** Add an extension's entries of one section to an output's, refusing a name described twice. */
function mergeSection<Entry>(
    section: string,
    target: Record<string, Entry>,
    entries: Readonly<Record<string, Entry>> | undefined,
): void {
    for (const [name, description] of Object.entries(entries ?? {})) {
        if (Object.hasOwn(target, name)) {
            throw new BuildError("BUILD_FAILED", `duplicate ${section} entry: ${name}`);
        }
        target[name] = description;
    }
}
