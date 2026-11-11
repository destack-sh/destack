import { isAbsolute, posix, resolve } from "node:path";
import { build, type InlineConfig, type Plugin, type PluginOption } from "vite";
import { rolldown } from "rolldown";
import { dynamicImportVarsPlugin, importGlobPlugin } from "rolldown/experimental";
import type {
    LogLevel,
    InputOptions,
    LogOrStringHandler,
    ObjectHook,
    OutputAsset,
    OutputOptions,
    OutputChunk,
    RolldownOptions,
    RolldownLog,
} from "rolldown";
import {
    DependencyName,
    DependencyRelease,
    type DependencyResolution,
    Package,
    type DeclarationDescription,
} from "@destack/package";
import {
    type BuildExtension,
    type TransformCache,
    type Compilation,
    type CompiledOutput,
    type DeclarationModule,
    HostModuleResolver,
    isHostModule,
    DESTACK_MODULE_SCHEME,
    isRuntimeModule,
    type OutputDescription,
    runtimeConditions,
    runtimeCss,
    runtimeTarget,
} from "@destack/package/build";
import type { BuildDescription } from "@destack/package/manifest";
import { PackageJson, type Sandbox } from "@destack/package";
import { PackagePath } from "@destack/package/file";
import { type OutputContent, type OutputExport } from "@destack/package/manifest";
import { type SourceMapReference } from "@destack/package/source";
import { modulePlugin } from "@destack/package/vite";
import type { ModuleTransformer } from "@destack/package/transform";
import { type Runtime } from "@destack/package/runtime";
import { type PackageSource } from "../source/index.ts";
import { relativePath } from "../source/dependency.ts";
import type { EvaluatedDeclaration } from "../declaration/index.ts";
import type { BuildDirectory } from "../build/directory.ts";
import { type ModuleOptions } from "../build/build.ts";
import { BuildError, extensionFailureOf, bundleFailureOf } from "../error/index.ts";
import { sourcePlugin, mapSource } from "./source.ts";
import { dependencyPlugin, modulePath, type ModuleSource } from "./dependency.ts";
import { ASSET, assetPlugin, directoryPlugin, moduleScriptPlugin, rawPlugin } from "./asset.ts";
import { transformPlugins } from "./extension.ts";

/** The directory of the chunks entries share, below an output's directory. */
const CHUNK_DIRECTORY = "chunk";

/** A stylesheet import, by its extension, with or without a query. */
const STYLESHEET = /\.css(?:\?.*)?$/u;

/** An import with a query, which a build plugin transforms and no runtime loads from a release. */
const QUERY = /\?/u;

/** The empty module standing in for a stylesheet a server output imports. */
const EMPTY_STYLESHEET = "\0destack-empty-stylesheet";

/** The runtime the compiler's transforms import for lowered code, bundled into each build. */
const TRANSFORM_HELPERS = "@oxc-project/runtime";

/** One output's build: its files, its description and the npm packages it imports. */
export interface OutputBuild {
    /** Dependencies, module imports, and generated file membership. */
    inspection: BuildDescription;
    /** The target output description. */
    output: OutputContent;
    /** The emitted file of each package export, recorded with its imports at the end. */
    exported: Record<string, string>;
    /** The emitted file of each installed command, recorded with its imports at the end. */
    bins: Record<string, string>;
    /** Generated files and captured source assets. */
    files: Map<string, Uint8Array<ArrayBuffer>>;
    /** Generated output paths written by Vite. */
    paths: string[];
    /** Maps for generated JavaScript. */
    sourceMaps: SourceMapReference[];
    /** The npm packages the output imports, which the host's link pins for its runtime. */
    npm: Set<string>;
}

/** The resolved dependencies, sources, destination and entries one compilation reads. */
interface CompileInput {
    /** The resolved dependencies, by name. */
    readonly dependencies: Readonly<Record<string, DependencyResolution>>;
    /** The source bytes the build snapshots once each, by package path. */
    readonly sources: Pick<BuildDirectory, "retained" | "retain">;
    /** The directory the build writes under. */
    readonly destinationRoot: string;
    /** The absolute source modules the output compiles. */
    readonly entries: readonly string[];
}

/** One module output a build compiles with the extensions of the package's dependency closure. */
export class OutputCompilation implements Compilation {
    /** The output's name. */
    readonly name: string;
    /** The output's settings. */
    readonly options: ModuleOptions;
    /** The opened package source. */
    readonly project: PackageSource;
    /** The package's declarations. */
    readonly declarations: readonly DeclarationDescription[];
    /** The module and export of each declaration, by its description. */
    readonly #exported: ReadonlyMap<DeclarationDescription, DeclarationModule>;
    /** The extensions of the package and its dependencies. */
    readonly extensions: readonly BuildExtension[];
    /** The build's cache of files the extensions' transforms derive. */
    readonly cache: TransformCache;
    /** The modules extensions emit, by package entrypoint. */
    readonly #entries = new Map<string, string>();
    /** The build's module transformer, stamping each module once across its passes. */
    readonly #transformer: ModuleTransformer;
    /** The host modules the runtime supplies, and those importers replace with nothing. */
    readonly #hosts: HostModuleResolver;

    /** Create the compilation of one output from its settings, package and inspection. */
    constructor(
        name: string,
        options: ModuleOptions,
        project: PackageSource,
        declarations: readonly EvaluatedDeclaration[],
        extensions: readonly BuildExtension[],
        cache: TransformCache,
        transformer: ModuleTransformer,
    ) {
        // store the output's settings, package and inspection
        this.name = name;
        this.options = options;
        this.project = project;
        this.declarations = declarations.map((declaration) => declaration.description);
        this.#exported = new Map(
            declarations.map((declaration) => [
                declaration.description,
                { file: declaration.file, export: declaration.export },
            ]),
        );
        this.extensions = extensions;
        this.cache = cache;
        this.#transformer = transformer;
        this.#hosts = new HostModuleResolver(project.runtime, project.environment.files);
    }

    /** The package the build compiles. */
    get package(): Package {
        return this.project.packageJson.package;
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

    /** What the package's workloads and views always use beyond the bare sandbox. */
    get sandbox(): Sandbox {
        return this.project.packageJson.definition.sandbox ?? {};
    }

    /** Locate the module and export of one of the package's declarations. */
    locate(declaration: DeclarationDescription): DeclarationModule {
        const exported = this.#exported.get(declaration);
        if (exported === undefined) {
            throw BuildError.of(
                "DECLARATION_INVALID",
                `declaration ${declaration.kind} '${declaration.name}' is invalid: it has no export`,
            );
        }

        return exported;
    }

    /** Emit a module as a chunk the package exports under an entrypoint. */
    entry(entrypoint: string, module: string): void {
        // refuse an entrypoint taken twice
        if (this.#entries.has(entrypoint) || Object.hasOwn(this.project.exports, entrypoint)) {
            throw BuildError.of(
                "ENTRY_INVALID",
                `entrypoint '${entrypoint}' is invalid: it is a duplicate`,
            );
        }
        this.#entries.set(entrypoint, module);
    }

    /** Compile the output with Vite and extension plugins, then let the extensions describe it. */
    async compile(
        dependencies: Readonly<Record<string, DependencyResolution>>,
        sources: Pick<BuildDirectory, "retained" | "retain">,
        destinationRoot: string,
    ): Promise<OutputBuild> {
        // collect the extensions' plugins and entries before compiling
        const { project } = this;
        const compiled = this.#createBuild();
        const plugins = this.#plugins();

        // export each stylesheet as the source file the build ships, compiling the modules alone
        const exported = Object.entries(project.exports);
        for (const [name, entry] of exported.filter(([, path]) => path.endsWith(".css"))) {
            compiled.exported[name] = PackagePath.parse(relativePath(project.directory, entry));
            compiled.files.set(
                compiled.exported[name],
                await project.environment.files.readFile(entry),
            );
        }
        const entries = [
            ...exported.map(([, entry]) => entry).filter((entry) => !entry.endsWith(".css")),
            ...Object.values(project.bins),
        ];
        if (!entries.length && !this.#entries.size) {
            this.#recordExports(compiled, new Map());

            return compiled;
        }
        if (entries.some((entry) => entry.endsWith(".html")) && project.runtime !== "browser") {
            throw BuildError.of(
                "ENTRY_INVALID",
                "HTML entry is invalid: entries in HTML require the browser runtime",
            );
        }

        // retain HTML before Vite rewrites its module and asset references
        await retainPages(project, entries, compiled.files);

        // compile a browser output with Vite, and a server output's modules with Rolldown alone
        const facades = new Map<string, string>();
        const input = { dependencies, sources, destinationRoot, entries };
        const generated =
            project.runtime === "browser"
                ? await runVite(this.#configuration(compiled, facades, plugins, input))
                : await runRolldown(await this.#serverOptions(compiled, facades, plugins, input));

        // map authored and emitted entries to generated files
        this.#collect(generated, compiled, facades);
        this.#exportEntries(generated, compiled.exported, facades);

        // let each extension describe the compiled output's workloads and views, then record the exports' imports
        this.#describeOutput(generated, compiled, facades);
        this.#recordExports(compiled, facades);

        return compiled;
    }

    /** Create the empty files and descriptions of the output under its output directory. */
    #createBuild(): OutputBuild {
        const output: OutputContent = {
            runtime: this.project.runtime,
            emit: true,
            workloads: {},
            views: {},
            directory: `output/${this.name}`,
            exports: {},
            dependencies: {},
            peers: {},
        };

        return {
            inspection: { packages: {}, inputs: {}, outputs: {} },
            output,
            exported: {},
            bins: {},
            files: new Map(),
            paths: [],
            sourceMaps: [],
            npm: new Set(),
        };
    }

    /** Let each extension describe the compiled output's workloads and views. */
    #describeOutput(
        generated: readonly (OutputChunk | OutputAsset)[],
        compiled: OutputBuild,
        facades: ReadonlyMap<string, string>,
    ): void {
        // expose the exports, and the declarations and stylesheets of each entrypoint
        const described: CompiledOutput = {
            exports: compiled.exported,
            declarations: (entrypoint) => this.#declarationsOf(entrypoint, compiled, facades),
            stylesheets: (entrypoint) => this.#stylesheetsOf(entrypoint, generated, compiled),
        };

        // merge each extension's description into the output
        for (const extension of this.extensions) {
            merge(compiled.output, describe(extension, this, described));
        }
    }

    /** Record each export's emitted file with the dependency specifiers its chunks import. */
    #recordExports(compiled: OutputBuild, facades: ReadonlyMap<string, string>): void {
        // walk a module entry's chunks, and record no imports of a stylesheet or a page
        for (const [name, path] of Object.entries(compiled.exported)) {
            const imports = facades.has(name) ? this.#importsOf(name, path, compiled) : [];
            compiled.output.exports[name] = { path, imports };
        }

        // record each command's emitted module with its imports, requiring it generated
        const bins: Record<string, OutputExport> = {};
        for (const command of Object.keys(this.project.bins)) {
            const path = compiled.bins[command];
            if (path === undefined) {
                throw BuildError.of("INVARIANT_BROKEN", `missing generated command: '${command}'`);
            }
            bins[command] = { path, imports: this.#importsOf(command, path, compiled) };
        }
        if (Object.keys(bins).length > 0) {
            compiled.output.bins = bins;
        }
    }

    /** Collect extension transforms before compiling plugins, reporting failures as build ones. */
    #plugins(): PluginOption[] {
        const compiling = this.extensions.flatMap((extension) => {
            try {
                return extension.compile?.(this) ?? [];
            } catch (cause) {
                throw extensionFailureOf(cause);
            }
        });

        const context = {
            directory: this.directory,
            runtime: this.runtime,
            // render components on the server runtimes, which run them without a document
            server: this.runtime !== "browser",
            dependencies: this.project.tree.directories,
            cache: this.cache,
            environment: this.project.environment,
        };

        return [...transformPlugins(this.extensions, context), ...compiling];
    }

    /** Configure Vite to compile the output's entries into the build directory. */
    #configuration(
        compiled: OutputBuild,
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
            // lower stylesheets to the baseline browsers too
            css: runtimeCss(project.runtime),
            ssr: {
                noExternal: true,
                // compile both server runtimes as module trees, their dependencies external
                target: "node",
                resolve: { conditions },
            },
            plugins: [
                // refuse a module script a browser module references as an asset, as no browser runs one
                ...(project.runtime === "browser" ? [moduleScriptPlugin()] : []),
                ...this.#packagePlugins(compiled, facades, input, locations),
                ...plugins,
                modulePlugin(this.#transformer),
            ],
            // give worker bundles their modules' metadata too
            worker: { plugins: () => [modulePlugin(this.#transformer)] },
            build: {
                emitAssets: true,
                write: true,
                outDir: destination,
                emptyOutDir: false,
                ssr: project.runtime !== "browser",
                // emit current syntax for the server runtimes Destack ships, and baseline syntax for browsers
                target: runtimeTarget(project.runtime),
                // minify the browser's modules as asked, leaving the server runtimes' as transpiled
                minify: project.runtime === "browser" && (options.minify ?? false),
                sourcemap: true,
                copyPublicDir: false,
                rolldownOptions: this.#rolldownOptions(compiled, input, destination, locations),
            },
        };
    }

    /** Configure Rolldown to compile a server output's modules into the build. */
    async #serverOptions(
        compiled: OutputBuild,
        facades: Map<string, string>,
        plugins: readonly PluginOption[],
        input: CompileInput,
    ): Promise<RolldownOptions & { readonly output: OutputOptions }> {
        // order the plugins as Vite does, the pre ones first, and call each as in a server build
        const { project } = this;
        const destination = resolve(input.destinationRoot, compiled.output.directory);
        const locations = new Map<string, ModuleSource>();
        const flattened = [
            serverStylePlugin(),
            rawPlugin(project.environment.files),
            assetPlugin(project.environment.files),
            ...this.#packagePlugins(compiled, facades, input, locations),
            ...(await flattenPlugins(plugins)),
            modulePlugin(this.#transformer),
        ];
        const ordered = [
            ...flattened.filter((plugin) => plugin.enforce === "pre"),
            ...flattened.filter((plugin) => plugin.enforce === undefined),
            ...flattened.filter((plugin) => plugin.enforce === "post"),
        ];

        // write the modules with the runtime's syntax and the production environment
        const options = this.#rolldownOptions(compiled, input, destination, locations);

        return {
            ...options,
            // expand template imports into literal ones over the files they match, as Vite's builds do
            plugins: [
                ...ordered.map((plugin) => serverPlugin(plugin)),
                dynamicImportVarsPlugin({ sourcemap: true }),
                importGlobPlugin(),
            ],
            transform: {
                target: runtimeTarget(project.runtime),
                define: { "process.env.NODE_ENV": JSON.stringify("production") },
            },
            output: { ...options.output, dir: destination, sourcemap: true },
        };
    }

    /** Create the plugins for the package's directories, sources, dependencies and entries. */
    #packagePlugins(
        compiled: OutputBuild,
        facades: Map<string, string>,
        input: CompileInput,
        locations: Map<string, ModuleSource>,
    ): Plugin[] {
        // share captured assets between the directory and dependency plugins
        const { project } = this;
        const assets = new Map<string, Uint8Array<ArrayBuffer>>();

        return [
            directoryPlugin(project.directory, compiled.files, assets, project.environment),
            sourcePlugin(
                project.directory,
                compiled.files,
                input.sources,
                input.destinationRoot,
                compiled.paths,
                project.environment,
            ),
            dependencyPlugin(
                project,
                input.dependencies,
                compiled.inspection,
                compiled.files,
                compiled.output.directory,
                project.runtime,
                undefined,
                locations,
                assets,
            ),
            this.#entryPlugin(facades),
            this.#hosts.plugin(),
        ];
    }

    /** Configure Rolldown's resolution, inputs, externals and generated file names. */
    #rolldownOptions(
        compiled: OutputBuild,
        input: CompileInput,
        destination: string,
        locations: ReadonlyMap<string, ModuleSource>,
    ): InputOptions & { readonly output: OutputOptions } {
        // resolve for the runtime's platform
        const { project } = this;
        const directory = compiled.output.directory;
        const isBun = project.runtime === "bun";
        const isBrowser = project.runtime === "browser";

        return {
            platform: isBun ? "node" : "browser",
            resolve: {
                mainFields: isBun ? ["module", "main"] : ["browser", "module", "main"],
                conditionNames: runtimeConditions(project.runtime),
            },
            cwd: project.directory,
            experimental: { attachDebugInfo: "none" },
            input: Object.fromEntries(
                input.entries.map((entry) => [entryName(project.directory, entry), entry]),
            ),
            preserveEntrySignatures: "strict",
            // ignore bundler timings
            checks: { bundlerTimings: false },
            onLog: failOnWarning,
            external: (specifier, importer) =>
                this.#isExternal(specifier, importer, input.dependencies, compiled),
            output: {
                format: "esm",
                // preserve function and class names in minified server modules, as error families and telemetry read them
                keepNames: !isBrowser,
                // bundle each entry at its path, sharing chunks between entries, other packages left to the import map
                entryFileNames: "[name].js",
                chunkFileNames: `${CHUNK_DIRECTORY}/[name]-[hash].js`,
                // name each asset by its file's name alone, wherever its source lives
                assetFileNames: ({ names: [name = "asset"] }) =>
                    `asset/${posix.basename(name, posix.extname(name))}-[hash][extname]`,
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
        compiled: OutputBuild,
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
                    compiled.exported[name] = path;
                    facades.set(name, source);
                }
                if (
                    entry.type === "asset" &&
                    source.endsWith(".html") &&
                    entry.fileName === relativePath(project.directory, source)
                ) {
                    compiled.exported[name] = path;
                }
            }

            // take a command's chunk
            for (const [command, source] of Object.entries(project.bins)) {
                if (entry.type === "chunk" && entry.facadeModuleId === source) {
                    compiled.bins[command] = path;
                }
            }

            // take a chunk's source map
            if (entry.type === "chunk" && entry.sourcemapFileName !== null) {
                compiled.sourceMaps.push({
                    generated: path,
                    map: `${directory}/${entry.sourcemapFileName}`,
                });
            }
        }
    }

    /** Export each emitted entry's chunk under its entrypoint, requiring every export. */
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
                throw BuildError.of("INVARIANT_BROKEN", `missing generated entry: '${entrypoint}'`);
            }
            exports[entrypoint] = PackagePath.parse(`output/${this.name}/${chunk.fileName}`);
        }

        // require every package export to be generated
        for (const name of Object.keys(this.project.exports)) {
            if (exports[name] === undefined) {
                throw BuildError.of("INVARIANT_BROKEN", `missing generated entry: '${name}'`);
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
                        fileName: `${name}.js`,
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

    /** Mark host modules and registry releases external, refusing undeclared dependencies. */
    #isExternal(
        specifier: string,
        importer: string | undefined,
        dependencies: Readonly<Record<string, DependencyResolution>>,
        compiled: Pick<OutputBuild, "output" | "npm">,
    ): boolean {
        // leave host modules to the host plugin, which keeps, empties or refuses them, and keep the Destack host's
        if (isHostModule(specifier)) {
            return false;
        } else if (specifier.startsWith(DESTACK_MODULE_SCHEME)) {
            return true;
        }

        // compile the package's own modules into the output, and a dependency's stylesheets, assets and queried files, which no runtime loads by import
        if (
            this.#isBundled(specifier) ||
            STYLESHEET.test(specifier) ||
            ASSET.test(specifier) ||
            QUERY.test(specifier)
        ) {
            return false;
        }

        // mark a declared dependency's release external
        return this.#isExternalRelease(specifier, importer, dependencies, compiled);
    }

    /** Check whether a specifier refers to a module of the package itself. */
    #isBundled(specifier: string): boolean {
        const name = DependencyName.of(specifier);

        return (
            specifier.startsWith(".") ||
            specifier.startsWith("/") ||
            specifier.startsWith("\0") ||
            specifier.startsWith("virtual:") ||
            specifier.startsWith("#") ||
            name === this.project.packageJson.package.name ||
            name === TRANSFORM_HELPERS
        );
    }

    /** Require a declared dependency, and pin its registry release as external. */
    #isExternalRelease(
        specifier: string,
        importer: string | undefined,
        dependencies: Readonly<Record<string, DependencyResolution>>,
        compiled: Pick<OutputBuild, "output" | "npm">,
    ): boolean {
        // require a declared dependency
        const dependency = DependencyName.of(specifier);
        const { packageJson } = this.project;
        const by = importer ?? packageJson.package.name;
        if (!PackageJson.runtimeDependencies(packageJson).includes(dependency)) {
            throw BuildError.of(
                "DEPENDENCY_INVALID",
                `runtime dependency '${dependency}' of '${by}' is invalid: it is undeclared`,
            );
        }

        // pin a Destack dependency's build, refusing an export without a module for the output's runtime
        const selected = dependencies[dependency];
        if (selected?.kind === "destack") {
            const modules = this.project.tree.export(specifier);
            if (modules !== undefined && modules[this.runtime] === undefined) {
                throw BuildError.of(
                    "RUNTIME_MODULE_INVALID",
                    `dependency '${specifier}' of '${by}' is invalid: it has no '${this.runtime}' module`,
                );
            }

            // record a builtin by its release, which every runtime links, and another build by its manifest
            compiled.output.dependencies[dependency] =
                this.project.tree.roles.get(dependency) === "builtin"
                    ? { kind: "builtin", package: Package.of(selected.package) }
                    : DependencyRelease.parse(selected);
        }
        // leave an npm package to the host's link, which pins its unit for the runtime
        else if (selected === undefined) {
            compiled.npm.add(dependency);
        }
        // refuse a source snapshot, which no runtime loads
        else {
            throw BuildError.of(
                "DEPENDENCY_INVALID",
                `runtime dependency '${dependency}' is invalid: it requires a registry release`,
            );
        }

        return true;
    }

    /** Select an entrypoint's module declarations, including ones shaken out of its chunks. */
    #declarationsOf(
        entrypoint: string,
        compiled: OutputBuild,
        facades: ReadonlyMap<string, string>,
    ): DeclarationDescription[] {
        // require an emitted entrypoint, whose chunks' imports recording its export checked
        const { inspection } = compiled;
        const emitted = compiled.exported[entrypoint];
        const facade = facades.get(entrypoint);
        if (emitted === undefined || facade === undefined) {
            throw BuildError.of("INVARIANT_BROKEN", `no emitted entrypoint: '${entrypoint}'`);
        }

        // start from the entry's module as the dependency plugin records it
        const root = isAbsolute(facade)
            ? relativePath(this.directory, facade)
            : facade.replaceAll(this.directory, ".");
        const sources = Object.entries(inspection.inputs)
            .filter(([, input]) => input.package === undefined && modulePath(input.path) === root)
            .map(([id]) => id);
        if (!sources.length) {
            throw BuildError.of("INVARIANT_BROKEN", `missing input of entrypoint: '${entrypoint}'`);
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

    /** List the stylesheets an entry's chunks import, statically or on demand, once each. */
    #stylesheetsOf(
        entrypoint: string,
        generated: readonly (OutputChunk | OutputAsset)[],
        compiled: OutputBuild,
    ): string[] {
        // start from the entry's chunk
        const directory = `${compiled.output.directory}/`;
        const emitted = compiled.exported[entrypoint];
        if (emitted === undefined) {
            throw BuildError.of("INVARIANT_BROKEN", `no emitted entrypoint: '${entrypoint}'`);
        }
        const chunks = new Map(
            generated.flatMap((entry) => (entry.type === "chunk" ? [[entry.fileName, entry]] : [])),
        );

        // collect each reached chunk's stylesheets, following its imports
        const pending = [emitted.slice(directory.length)];
        const visited = new Set<string>();
        const stylesheets = new Set<string>();
        for (let file = pending.shift(); file !== undefined; file = pending.shift()) {
            const chunk = chunks.get(file);
            if (chunk !== undefined && !visited.has(file)) {
                visited.add(file);
                for (const stylesheet of chunk.viteMetadata?.importedCss ?? []) {
                    stylesheets.add(`${directory}${stylesheet}`);
                }
                pending.push(...chunk.imports, ...chunk.dynamicImports);
            }
        }

        return [...stylesheets];
    }

    /** List the dependency specifiers an entry's chunks import, refusing ones nothing provides. */
    #importsOf(entrypoint: string, emitted: string, compiled: OutputBuild): string[] {
        // walk the entry's chunks from its emitted file
        const { inspection, output } = compiled;
        const chunks = [emitted];
        const visited = new Set<string>();
        const imports = new Set<string>();
        for (const path of chunks) {
            // visit each chunk once
            if (visited.has(path)) {
                continue;
            }
            visited.add(path);
            const chunk = inspection.outputs[path];
            if (!chunk) {
                throw BuildError.of("INVARIANT_BROKEN", `unknown emitted chunk: '${path}'`);
            }

            // follow internal imports and check external ones against the runtime and the dependencies
            for (const imported of chunk.imports) {
                const name = DependencyName.of(imported.path);
                const isDependency =
                    Object.hasOwn(output.dependencies, name) || compiled.npm.has(name);
                if (!imported.external) {
                    chunks.push(imported.path);
                } else if (isDependency) {
                    imports.add(imported.path);
                } else if (!isRuntimeModule(imported.path, this.runtime)) {
                    throw BuildError.of(
                        "RUNTIME_MODULE_INVALID",
                        `external import '${imported.path}' of '${entrypoint}' is invalid: it is not supported on '${this.runtime}'`,
                    );
                }
            }
        }

        return [...imports].toSorted();
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
                throw BuildError.of("INVARIANT_BROKEN", `unknown source module: '${id}'`);
            }

            // index the input's file under its package
            const owner =
                input.package === undefined
                    ? this.package
                    : inspection.packages[input.package]?.package;
            if (owner === undefined) {
                throw BuildError.of(
                    "INVARIANT_BROKEN",
                    `unknown source package: '${input.package}'`,
                );
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

/** Name an entry by its path below the package, its extension dropped but an HTML page's. */
function entryName(directory: string, entry: string): string {
    const path = relativePath(directory, entry);

    return path.endsWith(".html") ? path : path.replace(/\.[^./]+$/u, "");
}

/** Retain each HTML entry's authored bytes under its package path. */
async function retainPages(
    project: PackageSource,
    entries: readonly string[],
    files: Map<string, Uint8Array<ArrayBuffer>>,
): Promise<void> {
    for (const entry of entries.filter((path) => path.endsWith(".html"))) {
        const bytes = await project.environment.files.readFile(entry);
        files.set(relativePath(project.directory, entry), new Uint8Array(bytes));
    }
}

/** Run Vite and return the chunks and assets of its one output. */
async function runVite(configuration: InlineConfig): Promise<(OutputChunk | OutputAsset)[]> {
    // report a bundler failure as a build failure
    const generated = await build(configuration).catch((error: unknown) => {
        throw bundleFailureOf(error);
    });
    if (!("output" in generated)) {
        throw BuildError.of("INVARIANT_BROKEN", "expected one Vite output");
    }

    return generated.output;
}

/** Run Rolldown and return the chunks and assets it writes. */
async function runRolldown(
    options: RolldownOptions & { readonly output: OutputOptions },
): Promise<(OutputChunk | OutputAsset)[]> {
    // report a bundler failure as a build failure, closing the bundle either way
    let bundle: Awaited<ReturnType<typeof rolldown>> | undefined;
    try {
        bundle = await rolldown(options);
        const written = await bundle.write(options.output);

        return written.output;
    } catch (error) {
        throw bundleFailureOf(error);
    } finally {
        await bundle?.close();
    }
}

/** Load a server output's stylesheet imports as empty modules, as styles apply in browsers. */
function serverStylePlugin(): Plugin {
    return {
        name: "@destack/build/server-style",
        enforce: "pre",
        resolveId: {
            filter: { id: STYLESHEET },
            handler: () => EMPTY_STYLESHEET,
        },
        load: {
            filter: { id: /^\0destack-empty-stylesheet$/u },
            handler: () => "",
        },
    };
}

/** Flatten Vite plugin options into plugins, leaving out the empty ones. */
async function flattenPlugins(options: readonly PluginOption[]): Promise<Plugin[]> {
    const plugins: Plugin[] = [];
    for (const option of options) {
        const resolved = await option;
        if (Array.isArray(resolved)) {
            plugins.push(...(await flattenPlugins(resolved)));
        } else if (resolved !== false && resolved !== null && resolved !== undefined) {
            plugins.push(resolved);
        }
    }

    return plugins;
}

/** Call a Vite plugin's resolve, load and transform hooks as a Vite server build calls them. */
function serverPlugin(plugin: Plugin): Plugin {
    const { resolveId, load, transform } = plugin;

    return {
        ...plugin,
        ...(resolveId === undefined
            ? {}
            : {
                  resolveId: wrapHook(
                      resolveId,
                      (handler) =>
                          function (source, importer, options) {
                              return handler.call(this, source, importer, {
                                  ...options,
                                  ssr: true,
                              });
                          },
                  ),
              }),
        ...(load === undefined
            ? {}
            : {
                  load: wrapHook(
                      load,
                      (handler) =>
                          function (id, options) {
                              return handler.call(this, id, { ...options, ssr: true });
                          },
                  ),
              }),
        ...(transform === undefined
            ? {}
            : {
                  transform: wrapHook(
                      transform,
                      (handler) =>
                          function (code, id, options) {
                              return handler.call(this, code, id, {
                                  moduleType: options?.moduleType ?? "js",
                                  ...options,
                                  ssr: true,
                              });
                          },
                  ),
              }),
    };
}

/** Wrap a hook's handler, whether the hook is the handler itself or an object with its filter. */
function wrapHook<Handler extends (...parameters: never[]) => unknown, Meta>(
    hook: ObjectHook<Handler, Meta>,
    wrap: (handler: Handler) => Handler,
): ObjectHook<Handler, Meta> {
    return typeof hook === "object" ? { ...hook, handler: wrap(hook.handler) } : wrap(hook);
}

/** Fail on Rolldown warnings and pass other logs on. */
function failOnWarning(level: LogLevel, log: RolldownLog, handler: LogOrStringHandler): void {
    if (level === "warn") {
        throw BuildError.of("BUNDLE_INVALID", log.message);
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
        throw extensionFailureOf(cause);
    }
}

/** Add an extension's workloads and views to an output, refusing a name described twice. */
function merge(output: OutputContent, described: OutputDescription): void {
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
            throw BuildError.of(
                "ENTRY_INVALID",
                `'${section}' entry '${name}' is invalid: it is a duplicate`,
            );
        }
        target[name] = description;
    }
}
