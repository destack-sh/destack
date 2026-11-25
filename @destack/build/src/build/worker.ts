import { ModuleTransformer } from "@destack/package/transform";
import type { Program } from "@destack/package/bun";
import { Environment, FileSystemError, type Output, Telemetry } from "@destack/environment";
import { BuildTree } from "../dependency/tree.ts";
import { createHash } from "node:crypto";
import { dirname, join, resolve } from "node:path";
import { HOST_RUNTIME } from "@destack/package/runtime";
import { fileURLToPath } from "node:url";
import {
    graph,
    type PackageVersion,
    type DeclarationDescription,
    PackageJson,
    TEST_EXPORT,
} from "@destack/package";
import { Digest, found, type JsonObject, present, schema, Version } from "@destack/schema";
import { ObjectDescription } from "@destack/object";
import type { BuildExtension, TransformCache } from "@destack/package/build";
import { type SourceMapReference } from "@destack/package/source";
import { MANIFEST_LISTS } from "@destack/package/manifest";
import { PackageLocator } from "@destack/package/locator";
import type { Toolchain } from "@destack/check/toolchain";
import {
    ConstructorCatalog,
    Evaluation,
    type EvaluatedDeclaration,
    PackageCache,
} from "../declaration/index.ts";
import { DeclarationGraph } from "../graph/declaration.ts";
import {
    exportedDeclarations,
    Project,
    readPackageJson,
    type SourceOptions,
} from "../source/index.ts";
import { type BundleMode, ModuleOutput, type OutputBuild } from "../compile/output.ts";
import type { Bundle } from "../compile/bundle.ts";
import { loadExtensions } from "../compile/extension.ts";
import { checkInputs } from "../compile/runtime.ts";
import { Template } from "../template/index.ts";
import { DomainError } from "@destack/error";
import { BuildError, extensionFailureOf } from "../error/index.ts";
import { outputsOf, type PackageBuildOptions, type ModuleOptions } from "./build.ts";
import { BuildDirectory } from "./directory.ts";
import { readCatalogs } from "./catalog.ts";
import { describeGraph } from "../graph/module.ts";
import { ExampleDeclaration } from "../graph/example.ts";
import { ScenarioDeclaration } from "../graph/scenario.ts";
import { type OutlineModule, Outliner } from "../outline/outline.ts";
import { HostCache } from "./host-cache.ts";
import {
    type BuildRequest,
    type BuildResponse,
    HostMessage,
    readMessages,
    writeMessage,
    type RequestedBuild,
    type IndexedDeclarations,
} from "./message.ts";
import { Console } from "node:console";
import { Writable } from "node:stream";
import { PRIMARY_RUNTIMES } from "@destack/package/runtime";
import { TelemetryScope } from "@destack/telemetry";
import {
    InMemorySpanExporter,
    SimpleSpanProcessor,
    TracerProvider,
} from "@destack/telemetry/trace";
import { OtlpExport } from "@destack/telemetry/otlp";

/** A relative module specifier an emitted chunk imports, statically or dynamically. */
const RELATIVE_IMPORT = /(?:\bfrom\s*|\bimport\s*\(?\s*)["'](\.\.?\/[^"']+)["']/gu;

/** The Destack version this toolchain builds with: its own release's. */
const DESTACK_VERSION = import.meta.destack.package.version;

/** The Bun output a build compiles for its pages when it ships none, never distributed. */
const PRERENDER_OUTPUT = "prerender";

/** This package's directory, whose dependencies include the tools when running from a workspace. */
const PACKAGE = fileURLToPath(new URL("../..", import.meta.url));

/** What compiling one output reads: the request, projects, declarations and build. */
interface CompileContext {
    /** The requested build. */
    readonly options: RequestedBuild;
    /** Each output's opened project, by output name. */
    readonly projects: ReadonlyMap<string, Project>;
    /** The package's evaluated declarations. */
    readonly declarations: readonly EvaluatedDeclaration[];
    /** The extensions of the package's dependency closure. */
    readonly extensions: readonly BuildExtension[];
    /** The directory the build writes into. */
    readonly files: BuildDirectory;
    /** The build's cache of files the extensions' transforms derive. */
    readonly cache: TransformCache;
    /** The build's module transformer, stamping each module once across its outputs. */
    readonly transformer: ModuleTransformer;
    /** How the build's outputs bundle. */
    readonly mode: BundleMode;
}

/** The compiler inside the sandbox, retaining the last package's state across requests. */
export class CompilerWorker {
    /** The files, process and platform the compiler runs on. */
    readonly environment: Environment<"files" | "process" | "host">;
    /** The tools the compiler resolves named packages from. */
    readonly toolchain: Toolchain;
    /** The last build's module transformer, with the inputs every stamp depends on. */
    #transformed: { readonly inputs: string; readonly transformer: ModuleTransformer } | undefined;
    /** The Destack packages the build tool's dependencies resolve to. */
    readonly #packages: PackageLocator;
    /** The modules the compiler shares with the code it loads. */
    readonly #program: Program;
    /** The modules outlines read, kept across builds while each file's digest holds. */
    readonly #modules = new Map<string, OutlineModule>();
    /** The spans of the compiler's steps, recorded for the host. */
    readonly #scope: TelemetryScope;
    /** The build inputs the warm outputs and bundles started from, absent without any. */
    #warm: string | undefined;
    /** The warm outputs of the package in development, by output name, with their inputs. */
    readonly #outputs = new Map<string, { readonly key: string; readonly output: ModuleOutput }>();
    /** The warm bundles of the package's evaluated entries in development, by generated source. */
    readonly #evaluations = new Map<string, Bundle>();
    /** The cache of packages below stored tree roots, which never change. */
    readonly #stored = new PackageCache();

    /** Compile with a toolchain on an environment, sharing the linked modules. */
    constructor(
        program: Program,
        toolchain: Toolchain,
        environment: Environment<"files" | "process" | "host">,
    ) {
        // keep the environment, the program and the toolchain, then open the package locator
        this.environment = environment;
        this.#program = program;
        this.toolchain = toolchain;
        this.#packages = new PackageLocator(environment.files);
        this.#scope = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
    }

    /** Answer the host's requests on stdin with one retained compiler of a toolchain. */
    static async run(
        program: Program,
        toolchain: Toolchain,
        environment: Environment<"files" | "process" | "host">,
    ): Promise<void> {
        // reserve stdout for framed build results and send package logs to diagnostics
        const { stdin, stdout, stderr } = environment.process;
        const diagnostics = new Writable({
            write: (chunk: Uint8Array, _encoding, done: () => void) => {
                stderr.writeSync(chunk);
                done();
            },
        });
        Object.assign(
            globalThis.console,
            new Console({ stdout: diagnostics, stderr: diagnostics }),
            {
                write: (...messages: (string | ArrayBufferView | ArrayBuffer)[]) =>
                    writeDiagnostics(messages, stderr),
            },
        );
        const cache = new HostCache(stdout);

        // collect each request's ended spans, which leave with its answer as an OTLP export
        const ended = new InMemorySpanExporter();
        const tracers = new TracerProvider({
            spanProcessors: [new SimpleSpanProcessor({ exporter: ended })],
        });
        const telemetry = { ...environment.telemetry, tracers };
        const recording = Environment.instrument(environment, telemetry);
        const compiler = new CompilerWorker(program, toolchain, recording);
        const traces = () => {
            const spans = ended.getFinishedSpans();
            ended.reset();

            return OtlpExport.traces(spans).body;
        };

        // read the host's messages while requests run
        let responding = Promise.resolve();
        for await (const message of readMessages(stdin.readable, (value) =>
            HostMessage.parse(value),
        )) {
            // deliver the host's cache answers at once
            if (message.kind === "cached") {
                cache.answer(message);
            }
            // answer the host's requests one at a time
            else {
                responding = responding.then(async () => {
                    // answer it below the host's span, with the spans it ended
                    const parent = Telemetry.extract(new Headers(message.trace), telemetry);
                    const answer = await telemetry.context.with(parent, () =>
                        CompilerWorker.#respond(compiler, message.request, cache, traces),
                    );
                    await writeMessage(stdout, answer);
                });
            }
        }

        // fail the cache requests the host can no longer answer, then finish the requests
        cache.close(BuildError.of("COMPILER_BROKEN", "the host closed the compiler's input"));
        await responding;
    }

    /** Index a package from its syntax and its build's declarations: each module's graph file. */
    async index(
        options: SourceOptions & Pick<RequestedBuild, "tree">,
        declared: IndexedDeclarations,
    ): Promise<graph.Module[]> {
        // open the package, its declarations and relationships read from its build
        const project = await Project.open(options, this.environment, this.#program);
        const transformer = this.#transformer(options, project.packageJson);
        const graphed = DeclarationGraph.read(declared.declarations, declared.relationships);

        // outline each of its modules from their syntax
        const outliner = new Outliner(this.environment, this.#modules);
        const files = await Project.files(project.directory, this.environment);
        const outlines = [];
        for (const file of files) {
            outlines.push(await outliner.outline(file));
        }

        // read its examples, with the properties their modules evaluate, and its scenarios
        const definitions = outlines.flatMap((outline) => outline.definitions);
        const read = await Promise.all(
            definitions
                .filter((definition) => definition.kind === "example")
                .map((definition) => ExampleDeclaration.read(definition)),
        );
        const examples = await Evaluation.examples(project, read, transformer);
        const scenarios = await Promise.all(
            definitions
                .filter((definition) => definition.kind === "scenario")
                .map((definition) => ScenarioDeclaration.read(definition)),
        );

        // describe each module's graph file
        const source = project.packageJson.package;
        const tests = {
            package: await findPackage("@destack/test", this.#packages, this.toolchain),
            declarations: outlines.flatMap((outline) => outline.tests),
        };

        return describeGraph(source, outlines, graphed, tests, {
            package: await findPackage("@destack/package", this.#packages, this.toolchain),
            examples,
            scenarios,
        });
    }

    /** Evaluate a package's declarations from source, on its dependencies' builds. */
    async declarations(
        options: SourceOptions & Pick<RequestedBuild, "tree">,
    ): Promise<DeclarationDescription[]> {
        // open the package's source and evaluate its export entries
        const project = await Project.open(options, this.environment, this.#program);
        const catalog = await this.#catalog(options.versions ?? {}, options.tree);
        const transformer = this.#transformer(options, project.packageJson);
        const evaluation = await Evaluation.evaluate([project], catalog, { transformer });

        return evaluation.declarations.map((declaration) => declaration.description);
    }

    /** Evaluate a package's declarations, each export entry once, in the first project with it. */
    evaluate(options: RequestedBuild): Promise<EvaluatedDeclaration[]> {
        return this.#scope.span("destack.compiler.evaluate", {}, () => this.#evaluate(options));
    }

    /** Open the outputs' projects, then evaluate each export entry once. */
    async #evaluate(options: RequestedBuild): Promise<EvaluatedDeclaration[]> {
        // open the outputs' projects with the exports' on the runtime declarations evaluate on
        const packageJson = await readPackageJson(options.directory, this.environment.files);
        const [primary] = Object.entries(outputsOf(packageJson)).toSorted(
            ([, left], [, right]) =>
                PRIMARY_RUNTIMES.indexOf(left.runtime) - PRIMARY_RUNTIMES.indexOf(right.runtime),
        );
        const opened = await this.#open({
            ...options,
            outputs: {
                ...(primary === undefined ? {} : Object.fromEntries([primary])),
                ...options.outputs,
            },
        });

        // evaluate each export entry in the first project with it, on warm bundles in development
        const catalog = await this.#catalog(options.versions ?? {}, options.tree);
        const evaluated = new Set([opened.primary, ...opened.projects.values()]);
        const mode = await this.#mode(options, packageJson);
        const { declarations } = await Evaluation.evaluate([...evaluated.values()], catalog, {
            transformer: opened.transformer,
            ...(mode === "release" ? {} : { bundles: this.#evaluations }),
        });

        return [...declarations];
    }

    /** Open a build's catalog, sharing its tree's stored roots' cache across builds. */
    async #catalog(
        versions: Readonly<Record<string, Version>>,
        tree: Readonly<Record<string, string>>,
    ): Promise<ConstructorCatalog> {
        // match each root as the tree places it and as it really lies, as lookups read real paths
        const { files } = this.environment;
        const placed = Object.values(tree);
        const real = await Promise.all(placed.map((root) => files.realPath(root)));
        const roots = new Set([...placed, ...real]);

        // take the tree's roots, forgetting the entries of roots it no longer places
        this.#stored.retain(roots);

        return new ConstructorCatalog(versions, files, this.#stored);
    }

    /** Compile a package into a directory. */
    async build(
        options: RequestedBuild,
        declarations: readonly EvaluatedDeclaration[],
        destination: string,
        cache: TransformCache,
    ): Promise<void> {
        // retain the manifests, template and catalogs, then compile every output, a prerender output among them when pages render
        const opened = await this.#open(options, declarations);
        const files = new BuildDirectory(destination, this.environment);
        await retain(opened.primary, files);
        const compiled = await this.#compile(
            options,
            opened.planned,
            opened.projects,
            declarations,
            opened.extensions,
            files,
            cache,
            opened.transformer,
            await this.#mode(options, opened.packageJson),
        );

        // render the extensions' pages from the compiled outputs into the build, distributing no prerender output
        await this.#prerender(opened, compiled.outputs, options.tree, files);
        const internal = compiled.outputs.get(PRERENDER_OUTPUT);
        if (opened.isPrerenderOutput && internal !== undefined) {
            compiled.outputs.set(PRERENDER_OUTPUT, {
                ...internal,
                output: { ...internal.output, emit: false },
            });
        }

        // write the manifest
        const source = opened.primary.packageJson.package;
        const requires = PackageJson.ranges(opened.packageJson);
        const described = DeclarationGraph.describe(
            source,
            declarations.map((declaration) => declaration.description),
        );
        const tables = await this.#tables(source, described.list(), options.tree);
        await files.finish({
            destack: DESTACK_VERSION,
            package: source,
            role: opened.packageJson.definition.role,
            outputs: Object.fromEntries(
                [...compiled.outputs].map(([name, { output }]) => [name, output]),
            ),
            requires,
            sourceMaps: compiled.sourceMaps,
            declarations: described.list(),
            relationships: described.relationships(),
            tables,
            ...(opened.packageJson.definition.build === undefined
                ? {}
                : {
                      extension: await this.#extensionDigest(
                          opened.packageJson.definition.build,
                          compiled.outputs,
                          files.directory,
                      ),
                  }),
        });
    }

    /** Digest the modules a build extension's export loads, which its dependents' compiles run. */
    async #extensionDigest(
        build: string,
        outputs: ReadonlyMap<string, Pick<OutputBuild, "output">>,
        directory: string,
    ): Promise<Digest> {
        // find the export's file in the host runtime's output, which dependents' compiles import
        const subpath = build.slice(0, build.indexOf("#"));
        const entry = [...outputs.values()]
            .map(({ output }) => output)
            .find((output) => output.runtime === HOST_RUNTIME && subpath in output.exports)
            ?.exports[subpath]?.path;
        if (entry === undefined) {
            throw BuildError.of(
                "EXTENSION_INVALID",
                `build '${build}' is invalid: no ${HOST_RUNTIME} output exports '${subpath}'`,
            );
        }

        // walk its relative imports through the emitted chunks, hashing each module once
        const hash = createHash("sha256");
        const seen = new Set<string>();
        const pending = [entry];
        for (let path = pending.pop(); path !== undefined; path = pending.pop()) {
            if (!seen.has(path)) {
                seen.add(path);
                const text = await this.environment.files.readTextFile(join(directory, path));
                hash.update(`${path}\0${text}\0`);
                for (const [, specifier] of text.matchAll(RELATIVE_IMPORT)) {
                    pending.push(join(dirname(path), present(specifier, "a relative specifier")));
                }
            }
        }

        return Digest.parse(hash.digest("hex"));
    }

    /** Describe the tables the package's object types keep, with its dependencies' types. */
    async #tables(
        source: PackageVersion,
        declarations: readonly graph.Declaration[],
        tree: Readonly<Record<string, string>>,
    ): Promise<JsonObject> {
        // describe no tables for a package declaring no object types
        const own = ObjectDescription.installed(declarations);
        if (own.length === 0) {
            return {};
        }

        // read the object types the builds of its dependencies describe, from the tree it compiles against
        const { files } = this.environment;
        const directories = Object.entries(tree).filter(([name]) => name !== source.name);
        const depended = await Promise.all(
            directories.map(async ([, directory]) => {
                // pass a build without declarations, such as an npm package's
                const path = join(directory, MANIFEST_LISTS.declarations);
                if (!(await files.exists(path))) {
                    return [];
                }
                const listed = schema
                    .array(graph.Declaration)
                    .parse(JSON.parse(await files.readTextFile(path)));

                return ObjectDescription.installed(listed);
            }),
        );

        // describe its own tables, as a protocol's tables are the host's
        return ObjectDescription.tables([...depended.flat(), ...own], [source.id]).tables;
    }

    /** Read how a build bundles, dropping the warm outputs and bundles of other inputs. */
    async #mode(options: RequestedBuild, packageJson: PackageJson): Promise<BundleMode> {
        // build in development when the package's version is stamped so, keyed by the inputs the bundles start from
        const isDevelopment = options.versions?.[packageJson.package.name] === Version.development;
        const inputs = JSON.stringify([
            options.directory,
            options.dependencies,
            options.tree,
            options.versions ?? {},
            options.configuration ?? null,
            options.members ?? [],
        ]);
        const warm = isDevelopment ? inputs : undefined;

        // drop the warm outputs and bundles of other inputs
        if (this.#warm !== undefined && this.#warm !== warm) {
            for (const bundle of this.#evaluations.values()) {
                await bundle[Symbol.asyncDispose]();
            }
            for (const { output } of this.#outputs.values()) {
                await output[Symbol.asyncDispose]();
            }
            this.#evaluations.clear();
            this.#outputs.clear();
        }
        this.#warm = warm;

        return isDevelopment ? "development" : "release";
    }

    /** Take an output kept warm, starting anew once its options or declarations change. */
    async #warmOutput(
        name: string,
        output: ModuleOptions,
        context: CompileContext,
    ): Promise<ModuleOutput> {
        // reuse the output while its options and declarations hold
        const key = JSON.stringify([
            output,
            context.declarations.map((declaration) => declaration.description),
        ]);
        const warm = this.#outputs.get(name);
        if (warm?.key === key) {
            return warm.output;
        }

        // start a new one, stopping the stale one
        await warm?.output[Symbol.asyncDispose]();
        const created = createOutput(name, output, context);
        this.#outputs.set(name, { key, output: created });

        return created;
    }

    /** Plan a package's outputs with its closure's extensions, and open each output's project. */
    async #open(
        options: RequestedBuild,
        declarations: readonly EvaluatedDeclaration[] = [],
    ): Promise<{
        readonly packageJson: PackageJson;
        readonly extensions: readonly BuildExtension[];
        readonly rendering: readonly BuildExtension[];
        readonly declarations: readonly DeclarationDescription[];
        readonly isPrerenderOutput: boolean;
        readonly planned: ReadonlyMap<string, ModuleOptions>;
        readonly projects: ReadonlyMap<string, Project>;
        readonly primary: Project;
        readonly transformer: ModuleTransformer;
    }> {
        // plan the outputs with the extensions of the package's dependency closure
        const { files: system } = this.environment;
        const packageJson = await readPackageJson(options.directory, system);
        const tree = await BuildTree.read(options.tree, system);
        const loaded = await loadExtensions(options.directory, packageJson, tree, system);
        const extensions = loaded.map((entry) => entry.extension);
        const own = declarations
            .map((declaration) => declaration.description)
            .filter((declaration) => declaration.symbol.package.id === packageJson.package.id);
        const rendering = extensions.filter(
            (extension) => extension.prerender?.isRendering(own) === true,
        );
        const isPrerenderOutput =
            rendering.length > 0 &&
            !Object.values(options.outputs).some((output) => output.runtime === "bun");
        const modules = [...new Set(own.map((declaration) => declaration.symbol.symbol.module))];
        const planned = plan(options.outputs, isPrerenderOutput ? modules : undefined);

        // open each output's project
        const projects = new Map<string, Project>();
        for (const [name, output] of planned) {
            const isPrerender = isPrerenderOutput && name === PRERENDER_OUTPUT;
            projects.set(name, await this.#project(options, output, isPrerender));
        }
        const primary = found(projects, CompilerWorker.#primary(planned));
        const transformer = this.#transformer(options, packageJson);

        return {
            packageJson,
            extensions,
            rendering,
            declarations: own,
            isPrerenderOutput,
            planned,
            projects,
            primary,
            transformer,
        };
    }

    /** Reuse the last build's module transformer while the stamps' inputs stay the same. */
    #transformer(
        options: Pick<RequestedBuild, "tree" | "versions">,
        packageJson: PackageJson,
    ): ModuleTransformer {
        // key the stamps by the dependency builds, the release versions and the package's manifests
        const inputs = JSON.stringify([options.tree, options.versions ?? {}, packageJson]);
        if (this.#transformed?.inputs !== inputs) {
            const transformer = new ModuleTransformer(this.environment, {
                versions: options.versions ?? {},
            });
            this.#transformed = { inputs, transformer };
        }

        return this.#transformed.transformer;
    }

    /** Open the package with an output's entries. */
    #project(
        options: RequestedBuild,
        output: ModuleOptions,
        isPrerender = false,
    ): Promise<Project> {
        return Project.open(
            {
                directory: options.directory,
                ...(options.configuration === undefined
                    ? {}
                    : { configuration: options.configuration }),
                runtime: output.runtime,
                ...(output.entries === undefined ? {} : { entries: output.entries }),
                isPrerender,
                ...(options.versions === undefined ? {} : { versions: options.versions }),
                ...(options.members === undefined ? {} : { members: options.members }),
                tree: options.tree,
            },
            this.environment,
            this.#program,
        );
    }

    /** Pick the output whose program reads the whole source, by the runtime declarations run on. */
    static #primary(planned: ReadonlyMap<string, ModuleOptions>): string {
        // prefer Bun, where declarations evaluate, then workerd, then the browser
        const outputs = [...planned].toSorted(
            ([, left], [, right]) =>
                PRIMARY_RUNTIMES.indexOf(left.runtime) - PRIMARY_RUNTIMES.indexOf(right.runtime),
        );

        return present(outputs[0], "a planned output")[0];
    }

    /** Compile each output at once, joining their dependencies. */
    async #compile(
        options: RequestedBuild,
        planned: ReadonlyMap<string, ModuleOptions>,
        projects: ReadonlyMap<string, Project>,
        declarations: readonly EvaluatedDeclaration[],
        extensions: readonly BuildExtension[],
        files: BuildDirectory,
        cache: TransformCache,
        transformer: ModuleTransformer,
        mode: BundleMode,
    ): Promise<{
        outputs: Map<string, Pick<OutputBuild, "output" | "inspection" | "npm">>;
        sourceMaps: SourceMapReference[];
    }> {
        // compile every output at once
        const context = {
            options,
            projects,
            declarations,
            extensions,
            files,
            cache,
            transformer,
            mode,
        };
        const compiled = await Promise.all(
            [...planned].map(async ([name, output]) => {
                const built = await this.#module(name, output, context);

                return [name, built] as const;
            }),
        );

        // collect each output's description and every source map
        const outputs = new Map(compiled);
        const sourceMaps = compiled.flatMap(([, entry]) => entry.sourceMaps);

        return { outputs, sourceMaps };
    }

    /** Compile a module output with the extensions' plugins and write its files. */
    #module(
        name: string,
        output: ModuleOptions,
        context: CompileContext,
    ): Promise<Pick<OutputBuild, "output" | "inspection" | "npm" | "sourceMaps">> {
        const attributes = {
            "destack.package.directory": context.options.directory,
            "destack.compiler.output": name,
            "destack.runtime": output.runtime,
        };

        return this.#scope.span("destack.compiler.compile", attributes, () =>
            this.#compileModule(name, output, context),
        );
    }

    /** Compile an output, write its files and check the runtime of each module it loads. */
    async #compileModule(
        name: string,
        output: ModuleOptions,
        context: CompileContext,
    ): Promise<Pick<OutputBuild, "output" | "inspection" | "npm" | "sourceMaps">> {
        // compile the output, kept warm in development, and write its files
        const { options, files } = context;
        const compilation =
            context.mode === "release"
                ? createOutput(name, output, context)
                : await this.#warmOutput(name, output, context);
        const compiled = await compilation.compile(options.dependencies, files);
        await writeFiles(compiled.files, files);
        // check the runtime of each module a shipped output loads, as the prerender output ships none
        if (name !== PRERENDER_OUTPUT) {
            checkInputs(compiled.inspection, compilation.runtime);
        }

        return compiled;
    }

    /** Render the extensions' pages from the compiled outputs, their dependency builds linked. */
    async #prerender(
        opened: {
            readonly primary: Project;
            readonly rendering: readonly BuildExtension[];
            readonly declarations: readonly DeclarationDescription[];
        },
        outputs: ReadonlyMap<string, Pick<OutputBuild, "output" | "npm">>,
        tree: RequestedBuild["tree"],
        files: BuildDirectory,
    ): Promise<void> {
        // leave a build no extension renders pages of
        const { rendering } = opened;
        if (rendering.length === 0) {
            return;
        }

        // render in the prerender span
        await this.#scope.span("destack.compiler.prerender", {}, () =>
            this.#render(opened, outputs, tree, files),
        );
    }

    /** Link the dependency builds and npm packages, then write each extension's pages. */
    async #render(
        opened: {
            readonly primary: Project;
            readonly rendering: readonly BuildExtension[];
            readonly declarations: readonly DeclarationDescription[];
        },
        outputs: ReadonlyMap<string, Pick<OutputBuild, "output" | "npm">>,
        tree: RequestedBuild["tree"],
        files: BuildDirectory,
    ): Promise<void> {
        // read the extensions rendering pages
        const { rendering } = opened;

        // link the dependency builds and the outputs' npm packages, as installed beside the package
        const { files: system } = this.environment;
        const modules = join(files.directory, "node_modules");
        const linked = await BuildTree.read(tree, system);
        await linked.link(modules, system);
        const npm = new Set([...outputs.values()].flatMap((output) => [...output.npm]));
        for (const name of [...npm].filter((each) => linked.directories[each] === undefined)) {
            const installed = await system
                .realPath(join(opened.primary.directory, "node_modules", name))
                .catch(FileSystemError.absent);
            if (installed === undefined) {
                throw BuildError.of(
                    "DEPENDENCY_INVALID",
                    `npm package '${name}' is invalid: it renders the pages but is not installed beside the package`,
                );
            }
            await system.mkdir(dirname(join(modules, name)), { recursive: true });
            await system.symlink(installed, join(modules, name));
        }

        // render and write the pages
        try {
            const owner = opened.primary.packageJson.package;
            const context = {
                package: owner,
                directory: opened.primary.directory,
                build: files.directory,
                declarations: opened.declarations,
                outputs: Object.fromEntries(
                    [...outputs].map(([name, { output }]) => [name, output]),
                ),
                environment: this.environment,
            };
            for (const extension of rendering) {
                let pages: ReadonlyMap<string, Uint8Array<ArrayBuffer>>;
                try {
                    pages = (await extension.prerender?.render(context)) ?? new Map();
                } catch (cause) {
                    throw extensionFailureOf(cause);
                }
                for (const [path, bytes] of pages) {
                    await files.write(path, bytes);
                }
            }
        } finally {
            await system.remove(modules, { recursive: true });
        }
    }

    /** Run one request, answering its result or its failure. */
    static async #respond(
        compiler: CompilerWorker,
        request: BuildRequest,
        cache: TransformCache,
        traces: () => unknown,
    ): Promise<BuildResponse> {
        try {
            // evaluate the package's declarations, then build it on them
            if (request.kind === "build") {
                const declarations = await compiler.evaluate(request.options);
                await compiler.build(request.options, declarations, request.destination, cache);

                return {
                    kind: "result",
                    result: { kind: "build", declarations },
                    traces: traces(),
                };
            }
            // evaluate the package's declarations alone
            else if (request.kind === "evaluate") {
                const declarations = await compiler.evaluate(request.options);

                return {
                    kind: "result",
                    result: { kind: "evaluate", declarations },
                    traces: traces(),
                };
            }
            // build the package on declarations evaluated before
            else if (request.kind === "compile") {
                const { options, declarations, destination } = request;
                await compiler.build(options, declarations, destination, cache);

                return {
                    kind: "result",
                    result: { kind: "build", declarations },
                    traces: traces(),
                };
            }
            // index the package's code
            else if (request.kind === "index") {
                const modules = await compiler.index(request.options, request.declared);

                return {
                    kind: "result",
                    result: { kind: "index", modules },
                    traces: traces(),
                };
            }
            // evaluate the package's declarations from source
            else {
                const declarations = await compiler.declarations(request.options);

                return {
                    kind: "result",
                    result: { kind: "declarations", declarations },
                    traces: traces(),
                };
            }
        } catch (error) {
            return { kind: "result", error: describeFailure(error), traces: traces() };
        }
    }
}

/** Read a source's package.json as released: at its version, without the test layer. */
async function releasedManifest(project: Project): Promise<Uint8Array<ArrayBuffer>> {
    // read the authored manifest and its exports
    const path = resolve(project.directory, "package.json");
    const authored = await project.environment.files.readTextFile(path);
    const manifest = schema.jsonObject().parse(JSON.parse(authored));
    const exports = schema.jsonObject().parse(manifest["exports"]);
    const { version } = project.packageJson.package;

    // take the authored bytes when they list the version and no test layer
    const isTested = Object.hasOwn(exports, TEST_EXPORT);
    if (manifest["version"] === version && !isTested) {
        return new TextEncoder().encode(authored);
    }

    // otherwise list the release's version, and only the exports the build ships
    const shipped = Object.fromEntries(
        Object.entries(exports).filter(([name]) => name !== TEST_EXPORT),
    );
    const released = { ...manifest, version, exports: shipped };

    return new TextEncoder().encode(`${JSON.stringify(released, null, 4)}\n`);
}

/** Plan the requested module outputs and the prerender output, refusing invalid plans. */
function plan(
    requests: PackageBuildOptions["outputs"],
    prerendering: readonly string[] | undefined,
): Map<string, ModuleOptions> {
    // refuse an invalid name
    for (const name of Object.keys(requests)) {
        if (!/^[a-z][a-z0-9-]*$/u.test(name)) {
            throw BuildError.of(
                "OUTPUT_INVALID",
                `output name '${name}' is invalid: it must be a valid name`,
            );
        }
    }

    // refuse the prerender output's name, then add it when pages render and the build ships no Bun output
    const planned = new Map(Object.entries(requests));
    if (planned.has(PRERENDER_OUTPUT)) {
        throw BuildError.of(
            "OUTPUT_INVALID",
            `output '${PRERENDER_OUTPUT}' is invalid: it is the build's prerender output`,
        );
    }
    if (prerendering !== undefined) {
        const entries = Object.fromEntries(prerendering.map((module) => [`./${module}`, module]));
        planned.set(PRERENDER_OUTPUT, { kind: "module", runtime: "bun", entries });
    }

    // require at least one output
    if (!planned.size) {
        throw BuildError.of(
            "OUTPUT_INVALID",
            "build output set is invalid: a build requires at least one output",
        );
    }

    return planned;
}

/** Create the compilation of one planned output. */
function createOutput(name: string, output: ModuleOptions, context: CompileContext): ModuleOutput {
    // take the package's declarations the output's own exports make, beside its dependencies'
    const project = found(context.projects, name);
    const exported = new Set(Object.values(project.exports));
    const own = project.packageJson.package.id;
    const declarations = context.declarations.filter(
        (declaration) =>
            declaration.description.symbol.package.id !== own || exported.has(declaration.file),
    );

    return new ModuleOutput(
        name,
        output,
        context.mode,
        project,
        declarations,
        context.extensions,
        context.cache,
        context.transformer,
    );
}

/** Retain the package's template assets, manifests and catalogs before compiling. */
async function retain(project: Project, files: BuildDirectory): Promise<void> {
    // retain template assets, leaving package.json to its release's copy below
    if (project.packageJson.definition.template) {
        const template = await Template.read(project.directory, project.environment.files);
        for (const [path, bytes] of template.files) {
            if (path !== "package.json") {
                files.retain(path, new Uint8Array(bytes));
            }
        }
    }

    // retain the authored package declarations, package.json at the version the build releases
    files.retain("package.json", await releasedManifest(project));
    const system = project.environment.files;
    files.retain("destack.json", await system.readFile(resolve(project.directory, "destack.json")));

    // retain the declaration files the package exports, whichever outputs it builds
    for (const path of exportedDeclarations(project.packageJson)) {
        files.retain(path, await system.readFile(resolve(project.directory, path)));
    }

    // retain the package's catalogs
    const owner = project.packageJson.package.id;
    for (const [path, bytes] of await readCatalogs(project.directory, owner, system)) {
        files.retain(path, bytes);
    }
}

/** Write a compiled output's files into its build directory. */
async function writeFiles(
    generated: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
    files: BuildDirectory,
): Promise<void> {
    for (const [path, bytes] of generated) {
        await files.write(path, bytes);
    }
}

/** Read the identity of a Destack package among the build tool's dependencies. */
async function findPackage(
    name: string,
    locator: PackageLocator,
    toolchain: Toolchain,
): Promise<PackageVersion> {
    // find the package among the tools, refusing a missing one
    const directory = toolchain.locate([name], PACKAGE);
    const owner = await locator.find(join(directory, "package.json"));
    if (!owner) {
        throw BuildError.of("INVARIANT_BROKEN", `missing Destack package: '${name}'`);
    }

    return owner.metadata.package;
}

/** Describe a failure for the host, with a build failure's reason, its cause and its stack. */
function describeFailure(error: unknown): Extract<BuildResponse, { error: object }>["error"] {
    // describe a thrown value that is no error by its text
    if (!(error instanceof Error)) {
        const failure = BuildError.of("INVARIANT_BROKEN", String(error));

        return { failure: failure.toCallError(), audience: failure.audience };
    }

    // keep a family's failure with its audience, and report anything else as an internal fault
    const failure = DomainError.is(error)
        ? error
        : BuildError.of("INVARIANT_BROKEN", error.message);

    return {
        failure: failure.toCallError(),
        audience: failure.audience,
        cause: cloneableCause(error.cause),
        ...(error.stack === undefined ? {} : { stack: error.stack }),
    };
}

/** Serialize a failure's cause for the message, its text when it cannot be cloned. */
function cloneableCause(cause: unknown): unknown {
    try {
        return structuredClone(cause);
    } catch (error) {
        if (!(error instanceof DOMException && error.name === "DataCloneError")) {
            throw error;
        }

        return cause instanceof Error ? cause.message : String(cause);
    }
}

/** Write console output to diagnostics, returning the bytes written. */
function writeDiagnostics(
    messages: readonly (string | ArrayBufferView | ArrayBuffer)[],
    stderr: Output,
): number {
    // write at once, as the console's write answers the bytes written
    const bytes = Buffer.concat(messages.map((message) => toBuffer(message)));
    stderr.writeSync(bytes);

    return bytes.length;
}

/** View console output as bytes. */
function toBuffer(message: string | ArrayBufferView | ArrayBuffer): Buffer {
    // encode text
    if (typeof message === "string") {
        return Buffer.from(message);
    }
    // view a typed array's bytes
    else if (ArrayBuffer.isView(message)) {
        return Buffer.from(message.buffer, message.byteOffset, message.byteLength);
    }
    // view a buffer's bytes
    else {
        return Buffer.from(message);
    }
}
