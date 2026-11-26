import { TelemetryScope } from "@destack/telemetry";
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
    type DependencyRelease,
    PackageVersion,
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
import { PackageJson, type PackageId, type Sandbox } from "@destack/package";
import { PackagePath } from "@destack/package/file";
import { type PackageOutput, type PackageChunk } from "@destack/package/manifest";
import { type SourceMapReference } from "@destack/package/source";
import { baselinePlugin, modulePlugin } from "@destack/package/vite";
import type { ModuleTransformer } from "@destack/package/transform";
import { type Runtime } from "@destack/package/runtime";
import { type Project } from "../source/index.ts";
import { relativePath } from "../source/dependency.ts";
import type { EvaluatedDeclaration } from "../declaration/index.ts";
import type { BuildDirectory } from "../build/directory.ts";
import { type ModuleOptions } from "../build/build.ts";
import { BuildError, extensionFailureOf, bundleFailureOf } from "../error/index.ts";
import { sourcePlugin, mapSource } from "./source.ts";
import { dependencyPlugin, modulePath, type ModuleSource } from "./dependency.ts";
import { ASSET, assetPlugin, directoryPlugin, moduleScriptPlugin, rawPlugin } from "./asset.ts";
import { transformPlugins } from "./extension.ts";
import { Bundle, type BundleOutput } from "./bundle.ts";
import { BundleRecorder } from "./recorder.ts";

/** The directory of the chunks entries share, below an output's directory. */
const CHUNK_DIRECTORY = "chunk";

/** The directory of an output's assets and workers, below its directory. */
const ASSET_DIRECTORY = "asset";

/** A stylesheet import, by its extension, with or without a query. */
const STYLESHEET = /\.css(?:\?.*)?$/u;

/** An import with a query, which a build plugin transforms and no runtime loads from a release. */
const QUERY = /\?/u;

/** The Rolldown warning that plugins took a large share of a build, a timing report. */
const PLUGIN_TIMINGS = "PLUGIN_TIMINGS";

/** The empty module standing in for a stylesheet a server output imports. */
const EMPTY_STYLESHEET = "\0destack-empty-stylesheet";

/** A module path, by its extension. */
const MODULE = /\.[cm]?[jt]sx?$/u;

/** The encoder of generated text files. */
const ENCODER = new TextEncoder();

/** The runtime the compiler's transforms import for lowered code, bundled into each build. */
const TRANSFORM_HELPERS = "@oxc-project/runtime";

/** One output's build: its files, its description and the npm packages it imports. */
export interface OutputBuild {
    /** Dependencies, module imports, and generated file membership. */
    inspection: BuildDescription;
    /** The target output description. */
    output: PackageOutput;
    /** The emitted file of each package export, recorded with its imports at the end. */
    exported: Record<string, string>;
    /** The emitted file of each installed command, recorded with its imports at the end. */
    bins: Record<string, string>;
    /** Generated files and captured source assets. */
    files: Map<string, Uint8Array<ArrayBuffer>>;
    /** Maps for generated JavaScript. */
    sourceMaps: SourceMapReference[];
    /** The Destack packages its modules import, external to the output. */
    destack: Set<string>;
    /** The npm packages its modules import, external to the output. */
    npm: Set<string>;
}

/** How an output bundles: once per build, or warm across the builds of a package in development. */
export type BundleMode = "release" | "development";

/** An output's warm bundle and the state its plugins keep across its rebuilds. */
interface Session {
    /** The warm bundle. */
    readonly bundle: Bundle;
    /** The build the plugins record loaded files, captured assets and the inspection into. */
    readonly state: OutputBuild;
    /** The module starting each entry's chunk. */
    readonly facades: Map<string, string>;
    /** The bytes of each module the bundle loaded, by package path. */
    readonly sources: Map<string, Uint8Array<ArrayBuffer>>;
    /** The source file of each asset the bundle copies, by package path. */
    readonly assets: Map<string, string>;
    /** The directory the bundle names as its output, which it never writes. */
    readonly directory: string;
    /** The recorder of a browser bundle's stylesheets, absent for a server bundle. */
    readonly recorder: BundleRecorder | undefined;
    /** The output of the build that started the session, taken by the first rebuild. */
    first: BundleOutput | undefined;
}

/** The resolved dependencies, sources, destination and entries one compilation reads. */
interface CompileInput {
    /** The resolved dependencies, by name. */
    readonly dependencies: Readonly<Record<string, DependencyRelease>>;
    /** The source bytes the build snapshots once each, by package path. */
    readonly sources: Pick<BuildDirectory, "retained" | "retain" | "copy">;
    /** The directory the build writes under. */
    readonly destinationRoot: string;
    /** The absolute source modules the output compiles. */
    readonly entries: readonly string[];
    /** The recorder of what the bundle's builds reference. */
    readonly recorder: BundleRecorder;
}

/** One module output a build compiles with the extensions of the package's dependency closure. */
export class ModuleOutput implements Compilation {
    /** The output's name. */
    readonly name: string;
    /** The output's settings. */
    readonly options: ModuleOptions;
    /** How the output bundles. */
    readonly mode: BundleMode;
    /** The opened package source. */
    readonly project: Project;
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
    /** The output's warm bundle in development, once its first build starts it. */
    #session: Session | undefined;
    /** The spans and records of the output's bundles. */
    readonly #scope: TelemetryScope;

    /** Create the compilation of one output from its settings, package and inspection. */
    constructor(
        name: string,
        options: ModuleOptions,
        mode: BundleMode,
        project: Project,
        declarations: readonly EvaluatedDeclaration[],
        extensions: readonly BuildExtension[],
        cache: TransformCache,
        transformer: ModuleTransformer,
    ) {
        // store the output's settings, package and inspection
        this.name = name;
        this.options = options;
        this.mode = mode;
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
        this.#scope = TelemetryScope.of(import.meta.destack.package, project.environment.telemetry);
    }

    /** The package the build compiles. */
    get package(): PackageVersion {
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

    /** The names of the packages the package requires at run time. */
    get requires(): ReadonlySet<string> {
        return new Set(PackageJson.runtimeDependencies(this.project.packageJson));
    }

    /** The names of the Destack packages the package's modules resolve, by id. */
    get packages(): ReadonlyMap<PackageId, string> {
        return new Map([...this.project.tree.ids].map(([name, id]) => [id, name]));
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

    /** Compile the output into a build directory, then let the extensions describe it. */
    async compile(
        dependencies: Readonly<Record<string, DependencyRelease>>,
        files: BuildDirectory,
    ): Promise<OutputBuild> {
        // read the output's stylesheets and pages, leaving an output without modules
        const compiled = this.#createBuild();
        const entries = await this.#readEntries(compiled);
        if (entries === undefined) {
            this.#recordExports(compiled, new Map());

            return compiled;
        }

        // bundle once for a release, and on the warm bundle in development
        return this.mode === "release"
            ? await this.#compileOnce(dependencies, files, compiled, entries)
            : await this.#compileWarm(dependencies, files, compiled, entries);
    }

    /** Bundle a browser output with Vite and a server output with Rolldown, once. */
    async #compileOnce(
        dependencies: Readonly<Record<string, DependencyRelease>>,
        files: BuildDirectory,
        compiled: OutputBuild,
        entries: readonly string[],
    ): Promise<OutputBuild> {
        // bundle the entries with the extensions' plugins, which emit this bundle's entries anew
        const facades = new Map<string, string>();
        const plugins = this.#collectPlugins();
        const recorder = new BundleRecorder(this.directory, this.project.environment.files);
        const destinationRoot = files.directory;
        const input = { dependencies, sources: files, destinationRoot, entries, recorder };
        const generated =
            this.runtime === "browser"
                ? await runVite(this.#configuration(compiled, facades, plugins, input))
                : await runRolldown(await this.#serverOptions(compiled, facades, plugins, input));

        return this.#finish(generated, compiled, facades, importedStylesheets(generated));
    }

    /** Take the newest output of the warm bundle, started once, into a build directory. */
    async #compileWarm(
        dependencies: Readonly<Record<string, DependencyRelease>>,
        files: BuildDirectory,
        compiled: OutputBuild,
        entries: readonly string[],
    ): Promise<OutputBuild> {
        // take the newest output of the warm bundle
        const [session, { generated, modules, stylesheets }] = await this.#latest(
            dependencies,
            entries,
        );

        // keep the files of the modules still in the graph, the sources retained and the assets copied
        const { state, sources } = session;
        const live = new Set(modules.map((id) => relativePath(this.directory, modulePath(id))));
        const isLive = (path: string) => !MODULE.test(path) || live.has(path);
        for (const [path, bytes] of state.files) {
            if (isLive(path)) {
                compiled.files.set(path, bytes);
            }
        }
        for (const [path, bytes] of sources) {
            if (isLive(path)) {
                files.retain(path, bytes);
            }
        }
        for (const [path, source] of session.assets) {
            if (live.has(path)) {
                await files.copy(path, source);
            }
        }
        compiled.inspection = { ...state.inspection };
        compiled.destack = new Set(state.destack);
        compiled.npm = new Set(state.npm);

        return this.#finish(generated, compiled, session.facades, stylesheets);
    }

    /** Take the warm bundle's newest output, starting a bundle when none runs or one restyled. */
    async #latest(
        dependencies: Readonly<Record<string, DependencyRelease>>,
        entries: readonly string[],
    ): Promise<[Session, BundleOutput]> {
        // take the running bundle's output, started once
        const start = () =>
            this.#scope.span("destack.bundle.start", {}, () =>
                this.#startSession(dependencies, entries),
            );
        this.#session ??= await start();
        const running = this.#session;
        const isFirst = running.first !== undefined;
        const taken = await this.#output(running);

        // start a new bundle when a rebuild left the running one stale, taking its full first build
        if (!isFirst && (await running.recorder?.isStale()) === true) {
            await this[Symbol.asyncDispose]();
            this.#session = await start();
            const started = this.#session;

            return [started, await this.#output(started)];
        }

        return [running, taken];
    }

    /** Take a session's first output, else its bundle's newest, stopping a broken bundle. */
    async #output(session: Session): Promise<BundleOutput> {
        try {
            const output = await this.#scope.span("destack.bundle.latest", {}, async (span) => {
                // take the output, recording how many builds it took
                const before = session.bundle.builds;
                const taken = session.first ?? (await session.bundle.latest());
                span.setAttribute("destack.bundle.builds", session.bundle.builds - before);

                return taken;
            });
            session.first = undefined;

            return output;
        } catch (error) {
            // stop a broken bundle, so the next compile starts it again
            if (BuildError.is(error, "BUNDLE_BROKEN")) {
                await this[Symbol.asyncDispose]();
            }
            throw error;
        }
    }

    /** Stop the output's warm bundle and remove its output directory. */
    async [Symbol.asyncDispose](): Promise<void> {
        const session = this.#session;
        this.#session = undefined;
        if (session !== undefined) {
            await session.bundle[Symbol.asyncDispose]();
            await this.project.environment.files.remove(session.directory, { recursive: true });
        }
    }

    /** Start the output's warm bundle on the options a release build uses. */
    async #startSession(
        dependencies: Readonly<Record<string, DependencyRelease>>,
        entries: readonly string[],
    ): Promise<Session> {
        // record the loaded modules across rebuilds, reading each module anew in each rebuild
        const { files } = this.project.environment;
        const directory = await files.makeTemporaryDirectory("destack-develop-");
        const state = this.#createBuild();
        const facades = new Map<string, string>();
        const sources = new Map<string, Uint8Array<ArrayBuffer>>();
        const retained = new Map<string, Uint8Array<ArrayBuffer>>();
        const assets = new Map<string, string>();
        const retaining = {
            retained,
            retain: (path: string, bytes: Uint8Array<ArrayBuffer>) => {
                retained.set(path, bytes);
                sources.set(path, bytes);
            },
            copy: async (path: string, source: string) => {
                assets.set(path, source);
            },
        };
        const rebuilding: Plugin = {
            name: "@destack/build/rebuild",
            buildStart: () => retained.clear(),
        };

        // collect the plugins, which emit this bundle's entries anew
        const plugins = [rebuilding, ...this.#collectPlugins()];
        const recorder = new BundleRecorder(this.directory, files);
        const input = {
            dependencies,
            sources: retaining,
            destinationRoot: directory,
            entries,
            recorder,
        };

        // start a server output's bundle on its options, its first output from the bundle
        if (this.runtime !== "browser") {
            const options = await this.#serverOptions(state, facades, plugins, input);
            const { output, ...rest } = options;
            const bundle = await Bundle.start(rest, output, this.directory, files, recorder);

            return {
                bundle,
                state,
                facades,
                sources,
                assets,
                directory,
                recorder: undefined,
                first: undefined,
            };
        }

        // build a browser output with Vite once, capturing its final options for the bundle
        const captured: {
            input?: InputOptions;
            output?: OutputOptions;
            modules?: readonly string[];
        } = {};
        const capturing: Plugin = {
            name: "@destack/build/capture",
            enforce: "post",
            options(options) {
                captured.input = options;
            },
            outputOptions(options) {
                captured.output = options;
            },
            generateBundle() {
                captured.modules = [...this.getModuleIds()];
            },
        };
        const recording = [...plugins, capturing, recorder.vitePlugin()];
        const configuration = this.#configuration(state, facades, recording, input);
        const generated = await runVite(configuration);
        const { input: options, output, modules } = captured;
        if (options === undefined || output === undefined || modules === undefined) {
            throw BuildError.of("INVARIANT_BROKEN", "vite built without its bundler options");
        }
        recorder.clear();
        const bundle = await Bundle.start(options, output, this.directory, files, recorder);
        const first = { generated, modules, stylesheets: importedStylesheets(generated) };

        return { bundle, state, facades, sources, assets, directory, recorder, first };
    }

    /** Read the output's stylesheets and pages, returning its module entries if it has any. */
    async #readEntries(compiled: OutputBuild): Promise<string[] | undefined> {
        // export each stylesheet as the source file the build ships, compiling the modules alone
        const { project } = this;
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
            return undefined;
        }
        if (entries.some((entry) => entry.endsWith(".html")) && project.runtime !== "browser") {
            throw BuildError.of(
                "ENTRY_INVALID",
                "HTML entry is invalid: entries in HTML require the browser runtime",
            );
        }

        // retain HTML before Vite rewrites its module and asset references
        await retainPages(project, entries, compiled.files);

        return entries;
    }

    /** Map the generated files to the exports, then let the extensions describe the output. */
    #finish(
        generated: readonly (OutputChunk | OutputAsset)[],
        compiled: OutputBuild,
        facades: Map<string, string>,
        stylesheets: ReadonlyMap<string, readonly string[]>,
    ): OutputBuild {
        // map authored and emitted entries to generated files
        this.#collect(generated, compiled, facades);
        this.#exportEntries(generated, compiled.exported, facades);

        // let each extension describe the compiled output's workloads and views, then record the exports' imports
        this.#describeOutput(generated, compiled, facades, stylesheets);
        this.#recordExports(compiled, facades);

        return compiled;
    }

    /** Create the empty files and descriptions of the output under its output directory. */
    #createBuild(): OutputBuild {
        const output: PackageOutput = {
            runtime: this.project.runtime,
            emit: true,
            workloads: {},
            views: {},
            directory: `output/${this.name}`,
            exports: {},
            peers: {},
        };

        return {
            inspection: { packages: {}, inputs: {}, outputs: {} },
            output,
            exported: {},
            bins: {},
            files: new Map(),
            sourceMaps: [],
            destack: new Set(),
            npm: new Set(),
        };
    }

    /** Let each extension describe the compiled output's workloads and views. */
    #describeOutput(
        generated: readonly (OutputChunk | OutputAsset)[],
        compiled: OutputBuild,
        facades: ReadonlyMap<string, string>,
        stylesheets: ReadonlyMap<string, readonly string[]>,
    ): void {
        // expose the exports, and the declarations and stylesheets of each entrypoint
        const described: CompiledOutput = {
            exports: compiled.exported,
            declarations: (entrypoint) => this.#declarationsOf(entrypoint, compiled, facades),
            stylesheets: (entrypoint) =>
                this.#stylesheetsOf(entrypoint, generated, compiled, stylesheets),
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
        const bins: Record<string, PackageChunk> = {};
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

    /** Collect the extensions' transforms and plugins anew, the extensions emitting the entries. */
    #collectPlugins(): PluginOption[] {
        // forget the entries the extensions emitted before, as they emit them again
        this.#entries.clear();

        // collect each extension's plugins, reporting its failure as the build's
        const compiling = this.extensions.flatMap((extension) => {
            try {
                return extension.compile?.(this) ?? [];
            } catch (cause) {
                throw extensionFailureOf(cause);
            }
        });

        // transform with the context the output builds in
        const context = {
            directory: this.directory,
            runtime: this.runtime,
            // render components on the server runtimes, which run them without a document
            server: this.runtime !== "browser",
            mode: this.mode,
            dependencies: this.project.tree.builds,
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
                // refuse module scripts referenced as assets, and run the browser baseline first in each entry
                ...(project.runtime === "browser" ? [moduleScriptPlugin(), baselinePlugin()] : []),
                ...this.#packagePlugins(compiled, facades, input, locations),
                ...plugins,
                modulePlugin(this.#transformer),
            ],
            // give worker bundles their modules' metadata and the browser baseline too
            // NOTE #Incomplete: a warm rebuild leaves a worker stale when a module it shares with the page changes
            worker: { plugins: () => [modulePlugin(this.#transformer), baselinePlugin()] },
            build: {
                emitAssets: true,
                // emit the files Vite names itself, such as a worker's, in the output's asset directory
                assetsDir: ASSET_DIRECTORY,
                write: false,
                outDir: destination,
                emptyOutDir: false,
                ssr: project.runtime !== "browser",
                // emit current syntax for the server runtimes Destack ships, and baseline syntax for browsers
                target: runtimeTarget(project.runtime),
                // minify the browser's modules in a release unless asked otherwise, leaving the server runtimes' as transpiled
                minify:
                    project.runtime === "browser" && (options.minify ?? this.mode === "release"),
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
            assetPlugin(project.environment.files, input.recorder),
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
            directoryPlugin(
                project.directory,
                compiled.files,
                assets,
                project.environment,
                input.recorder,
            ),
            input.recorder.plugin(),
            sourcePlugin(
                project.directory,
                project.packageJson.package.name,
                compiled.files,
                input.sources,
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
            // record Rolldown's report of slow plugins on the compile's span
            onLog: (level, log, handler) => {
                failOnWarning(level, log, handler, this.#scope);
            },
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
                    `${ASSET_DIRECTORY}/${posix.basename(name, posix.extname(name))}-[hash][extname]`,
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
            // keep the generated file's bytes
            const path = PackagePath.parse(`${directory}/${entry.fileName}`);
            const content = entry.type === "chunk" ? entry.code : entry.source;
            compiled.files.set(
                path,
                typeof content === "string" ? ENCODER.encode(content) : Uint8Array.from(content),
            );

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
        dependencies: Readonly<Record<string, DependencyRelease>>,
        compiled: Pick<OutputBuild, "destack" | "npm">,
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

    /** Require a declared dependency, and mark its Destack release as external. */
    #isExternalRelease(
        specifier: string,
        importer: string | undefined,
        dependencies: Readonly<Record<string, DependencyRelease>>,
        compiled: Pick<OutputBuild, "destack" | "npm">,
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

        // refuse the package's own import of an internal export, the build's generated entries aside
        if (importer?.startsWith(`${this.project.directory}/`) === true) {
            this.project.tree.refuseInternal(specifier);
        }

        // keep a Destack dependency external
        const selected = dependencies[dependency];
        if (selected?.kind === "destack") {
            const modules = this.project.tree.export(specifier);
            if (modules !== undefined && modules[this.runtime] === undefined) {
                throw BuildError.of(
                    "RUNTIME_MODULE_INVALID",
                    `dependency '${specifier}' of '${by}' is invalid: it has no '${this.runtime}' module`,
                );
            }
            compiled.destack.add(dependency);
        }
        // leave an npm package to the dependency tree, which links its unit
        else if (selected === undefined) {
            compiled.npm.add(dependency);
        }
        // refuse an npm unit's build selected as a dependency, which the tree links instead
        else {
            throw BuildError.of(
                "DEPENDENCY_INVALID",
                `runtime dependency '${dependency}' is invalid: it is an npm unit, which the dependency tree links`,
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
        imported: ReadonlyMap<string, readonly string[]>,
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
                for (const stylesheet of imported.get(file) ?? []) {
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
        const { inspection } = compiled;
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
                const isDependency = compiled.destack.has(name) || compiled.npm.has(name);
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
    project: Project,
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

/** Run Rolldown and return the chunks and assets it generates. */
async function runRolldown(
    options: RolldownOptions & { readonly output: OutputOptions },
): Promise<(OutputChunk | OutputAsset)[]> {
    // report a bundler failure as a build failure, closing the bundle either way
    let bundle: Awaited<ReturnType<typeof rolldown>> | undefined;
    try {
        bundle = await rolldown(options);
        const generated = await bundle.generate(options.output);

        return generated.output;
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

/** Fail on Rolldown warnings other than timings, recording timings on the active span. */
function failOnWarning(
    level: LogLevel,
    log: RolldownLog,
    handler: LogOrStringHandler,
    scope: TelemetryScope,
): void {
    // record the plugin timings on the compile's span
    if (log.code === PLUGIN_TIMINGS) {
        scope.span.current()?.setAttribute("destack.bundle.plugins", log.message);
    }
    // fail on any other warning
    else if (level === "warn") {
        throw BuildError.of("BUNDLE_INVALID", log.message);
    }
    // pass other logs on
    else {
        handler(level, log);
    }
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

/** Add an extension's workloads, views and frame to an output, refusing one described twice. */
function merge(output: PackageOutput, described: OutputDescription): void {
    // add the workloads and the views
    mergeSection("workloads", output.workloads, described.workloads);
    mergeSection("views", output.views, described.views);

    // add the frame, refusing a second
    if (described.frame !== undefined && output.frame !== undefined) {
        throw BuildError.of("ENTRY_INVALID", "'frame' entry is invalid: it is a duplicate");
    } else if (described.frame !== undefined) {
        output.frame = described.frame;
    }
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

/** Read the stylesheets each chunk of a full build imports, as Vite's CSS plugin records them. */
function importedStylesheets(
    generated: readonly (OutputChunk | OutputAsset)[],
): ReadonlyMap<string, readonly string[]> {
    return new Map(
        generated.flatMap((entry) =>
            entry.type === "chunk" && entry.viteMetadata !== undefined
                ? [[entry.fileName, [...entry.viteMetadata.importedCss]] as const]
                : [],
        ),
    );
}
