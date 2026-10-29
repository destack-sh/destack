import { ModuleTransformer } from "@destack/package/transform";
import { type Environment, type Output } from "@destack/environment";
import { DependencyTree } from "../dependency/tree.ts";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
    type DependencyResolution,
    graph,
    type Package,
    type DeclarationDescription,
    type PackageJson,
} from "@destack/package";
import { nth, found, present, schema } from "@destack/schema";
import {
    type BuildExtension,
    type ExpandedOutput,
    type OutputKind,
    type OutputRequest,
    type TransformCache,
} from "@destack/package/build";
import { type PackageOutput } from "@destack/package/manifest";
import { type SourceMapReference } from "@destack/package/source";
import { PackageLocator } from "@destack/package/locator";
import { Toolchain } from "@destack/check";
import { type TypeScriptInspection, TypeScriptCompiler } from "../typescript/index.ts";
import { inspectModules, type InspectOptions } from "../inspection/inspection.ts";
import { ConstructorCatalog, Evaluation, type EvaluatedDeclaration } from "../declaration/index.ts";
import { DeclarationGraph } from "../graph/declaration.ts";
import { PackageSource, readPackageJson } from "../source/index.ts";
import { OutputCompilation, type OutputBuild } from "../compile/compilation.ts";
import { loadExtensions } from "../compile/extension.ts";
import { OutputPass } from "../compile/pass.ts";
import { checkInputs, RuntimeCompiler } from "../compile/runtime.ts";
import { Template } from "../template/index.ts";
import { DomainError } from "@destack/error";
import { BuildError, extensionFailureOf } from "../error/index.ts";
import { outputsOf, type PackageBuildOptions, type ModuleOptions } from "./build.ts";
import { BuildDirectory } from "./directory.ts";
import { readCatalogs } from "./catalog.ts";
import { comparePath } from "./serialization.ts";
import { describeCode, describeGraph } from "../graph/module.ts";
import { HostCache } from "./host-cache.ts";
import {
    type BuildRequest,
    type BuildResponse,
    HostMessage,
    readMessages,
    writeMessage,
    type RequestedBuild,
} from "./message.ts";
import { Console } from "node:console";
import { Writable } from "node:stream";
import { PRIMARY_RUNTIMES, type Runtime } from "@destack/package/runtime";
import type { ModuleDescription } from "@destack/package/code";

/** This package's directory, whose dependencies include the tools when running from a workspace. */
const PACKAGE = fileURLToPath(new URL("../..", import.meta.url));

/** An output a build compiles: a module output, or one output of a kind's pass. */
interface PlannedOutput {
    /** The output's module settings. */
    readonly options: ModuleOptions;
    /** Package-relative modules inspected beside the entries. */
    readonly files: readonly string[];
    /** The kind's request compiling the output in a separate pass. */
    readonly pass?: {
        /** The requested output's name, shared by the outputs the kind expands it into. */
        readonly name: string;
        /** The output kind compiling the pass. */
        readonly kind: OutputKind;
        /** The request with the kind's settings. */
        readonly request: OutputRequest;
    };
}

/** What compiling one output or pass reads: the request, projects, declarations and build. */
interface CompileContext {
    /** The requested build. */
    readonly options: RequestedBuild;
    /** Each output's opened project, by output name. */
    readonly projects: ReadonlyMap<string, PackageSource>;
    /** The package's evaluated declarations. */
    readonly declarations: readonly EvaluatedDeclaration[];
    /** The extensions of the package's dependency closure. */
    readonly extensions: readonly BuildExtension[];
    /** The directory the build writes into. */
    readonly files: BuildDirectory;
    /** The build's cache of files the extensions' transforms derive. */
    readonly cache: TransformCache;
    /** The build's module transformer, stamping each module once across its passes. */
    readonly transformer: ModuleTransformer;
}

/** The compiler inside the sandbox, retaining the last package's state across requests. */
export class CompilerWorker implements AsyncDisposable {
    /** The files, process and platform the compiler runs on. */
    readonly environment: Environment<"files" | "process" | "host">;
    /** The last build's module transformer, with the inputs every stamp depends on. */
    #transformed: { readonly inputs: string; readonly transformer: ModuleTransformer } | undefined;
    /** The native TypeScript compiler of the last requested package. */
    #typescript: TypeScriptCompiler | undefined;
    /** Runtime type environments reused across output checks. */
    readonly runtimes: RuntimeCompiler;
    /** The Destack packages the build tool's dependencies resolve to. */
    readonly #packages: PackageLocator;

    /** Compile on an environment's files, process and platform. */
    constructor(environment: Environment<"files" | "process" | "host">) {
        this.environment = environment;
        this.runtimes = new RuntimeCompiler(environment);
        this.#packages = new PackageLocator(environment.files);
    }

    /** Switch to a package's native compiler, closing the last package's. */
    async #switch(directory: string): Promise<TypeScriptCompiler> {
        // reuse the compiler of the package the last request listed
        const current = this.#typescript;
        if (current?.directory === directory) {
            return current;
        }

        // close the last package's compiler
        await current?.[Symbol.asyncDispose]();

        // start the package's compiler
        this.#typescript = await TypeScriptCompiler.start(directory, this.environment);

        return this.#typescript;
    }

    /** Answer the host's requests on stdin with one retained compiler. */
    static async run(environment: Environment<"files" | "process" | "host">): Promise<void> {
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
        await using compiler = new CompilerWorker(environment);

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
                responding = responding.then(async () =>
                    writeMessage(stdout, await CompilerWorker.#respond(compiler, message, cache)),
                );
            }
        }

        // fail the cache requests the host can no longer answer, then finish the requests
        cache.close(BuildError.of("COMPILER_FAILED", "the host closed the compiler's input"));
        await responding;
    }

    /** Inspect source using retained compiler state and source configurations. */
    async inspect(options: InspectOptions & Pick<RequestedBuild, "tree">) {
        const { project, inspection, declarations } = await this.#inspectPackage(options);

        return inspectModules(
            project.packageJson.package,
            inspection.modules,
            inspection.tests,
            declarations,
        );
    }

    /** Index source completely: every module's symbols and code edges, and the module graph. */
    async index(
        options: InspectOptions & Pick<RequestedBuild, "tree">,
    ): Promise<{ code: graph.Document[]; modules: graph.Module[] }> {
        // inspect the package, and evaluate its declarations and its examples' properties
        const { project, inspection, declarations, transformer } =
            await this.#inspectPackage(options);
        const examples = await Evaluation.examples(project, inspection.examples, transformer);
        const source = project.packageJson.package;
        const tests = {
            package: await findPackage("@destack/test", this.#packages, this.environment),
            declarations: inspection.tests,
        };

        // describe the code and the module graph
        return {
            code: describeCode(source, inspection.modules, declarations, tests),
            modules: describeGraph(source, inspection.modules, declarations, tests, {
                package: await findPackage("@destack/package", this.#packages, this.environment),
                examples,
                scenarios: inspection.scenarios,
            }),
        };
    }

    /** Inspect the whole package with its dependencies' builds and evaluate its declarations. */
    async #inspectPackage(options: InspectOptions & Pick<RequestedBuild, "tree">): Promise<{
        project: PackageSource;
        inspection: TypeScriptInspection;
        declarations: DeclarationDescription[];
        transformer: ModuleTransformer;
    }> {
        // inspect source with the selected compiler configuration and the dependencies' builds
        const project = await PackageSource.open(options, this.environment);
        const typescript = await this.#switch(options.directory);
        const inspection = await typescript.inspect(project);

        // evaluate the package's declarations
        const catalog = new ConstructorCatalog({}, this.environment.files);
        const transformer = new ModuleTransformer(this.environment.files);
        const evaluation = await Evaluation.evaluate([project], catalog, {
            isIndexing: true,
            transformer,
        });
        const declarations = evaluation.declarations.map((declaration) => declaration.description);

        return { project, inspection, declarations, transformer };
    }

    /** Evaluate a package's declarations, each export entry once, in the first project with it. */
    async evaluate(options: RequestedBuild): Promise<EvaluatedDeclaration[]> {
        // open the outputs' projects, then evaluate each export entry in the first project with it
        const opened = await this.#open(options);
        const catalog = new ConstructorCatalog(options.versions ?? {}, this.environment.files);
        const evaluated = new Set([opened.primary, ...opened.projects.values()]);
        const { declarations } = await Evaluation.evaluate([...evaluated.values()], catalog, {
            isIndexing: false,
            transformer: opened.transformer,
        });

        return [...declarations];
    }

    /** Check each output's program: its type diagnostics and its modules' globals per runtime. */
    async check(options: RequestedBuild): Promise<void> {
        // inspect each requested output's program, then each the package's exports ship to
        const packageJson = await readPackageJson(options.directory, this.environment.files);
        const exported = Object.entries(outputsOf(packageJson)).filter(
            ([name]) => !Object.hasOwn(options.outputs, name),
        );
        const opened = await this.#open({
            ...options,
            outputs: { ...options.outputs, ...Object.fromEntries(exported) },
        });
        const typescript = await this.#switch(options.directory);
        const shipped = new Map<string, { module: ModuleDescription; runtimes: Set<Runtime> }>();
        for (const project of new Set(opened.projects.values())) {
            // refuse the program's diagnostics, and check the globals module evaluation reads
            const { modules } = await typescript.inspect(project);
            const evaluated = modules.flatMap((module) =>
                module.globals.filter((global) => global.evaluation === "module"),
            );
            await this.runtimes.check(evaluated, [project.runtime]);

            // note the runtimes each module ships to
            for (const module of modules) {
                const entry = shipped.get(module.path) ?? { module, runtimes: new Set() };
                entry.runtimes.add(project.runtime);
                shipped.set(module.path, entry);
            }
        }

        // check the globals functions read against any runtime their module ships to
        for (const { module, runtimes } of shipped.values()) {
            const deferred = module.globals.filter((global) => global.evaluation !== "module");
            await this.runtimes.check(deferred, [...runtimes]);
        }
    }

    /** Compile a package into a directory, answering its outputs' npm imports. */
    async build(
        options: RequestedBuild,
        declarations: readonly EvaluatedDeclaration[],
        destination: string,
        cache: TransformCache,
    ): Promise<Record<string, string[]>> {
        // retain the manifests, template and catalogs, then compile every output
        const opened = await this.#open(options);
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
        );

        // write the manifest: the outputs, the sources and releases they bundle, and the declarations
        const source = opened.primary.packageJson.package;
        const graph = new DeclarationGraph(
            source,
            declarations.map((declaration) => declaration.description),
        );
        await files.finish({
            package: source,
            ...(options.commit === undefined ? {} : { commit: options.commit }),
            outputs: Object.fromEntries(
                [...compiled.outputs].map(([name, { output }]) => [name, output]),
            ),
            dependencies: compiled.resolved,
            sourceMaps: compiled.sourceMaps,
            declarations: graph.list(),
            isExtension: opened.packageJson.definition.build !== undefined,
        });

        return Object.fromEntries(
            [...compiled.outputs].map(([name, { npm }]) => [name, [...npm].toSorted()]),
        );
    }

    /** Plan a package's outputs with its closure's extensions, and open each output's project. */
    async #open(options: RequestedBuild): Promise<{
        readonly packageJson: PackageJson;
        readonly extensions: readonly BuildExtension[];
        readonly planned: ReadonlyMap<string, PlannedOutput>;
        readonly projects: ReadonlyMap<string, PackageSource>;
        readonly primary: PackageSource;
        readonly transformer: ModuleTransformer;
    }> {
        // plan the outputs with the extensions of the package's dependency closure
        const { files: system } = this.environment;
        const packageJson = await readPackageJson(options.directory, system);
        const tree = await DependencyTree.read(options.tree, system);
        const loaded = await loadExtensions(options.directory, packageJson, tree, system);
        const extensions = loaded.map((entry) => entry.extension);
        const planned = expand(options.outputs, extensions);

        // open each output's project
        const projects = new Map<string, PackageSource>();
        for (const [name, output] of planned) {
            projects.set(name, await this.#project(options, output));
        }
        const primary = found(projects, CompilerWorker.#primary(planned));
        const transformer = this.#transformer(options, packageJson);

        return { packageJson, extensions, planned, projects, primary, transformer };
    }

    /** Reuse the last build's module transformer while the stamps' inputs stay the same. */
    #transformer(options: RequestedBuild, packageJson: PackageJson): ModuleTransformer {
        // key the stamps by the dependency builds, the release versions and the package's manifests
        const inputs = JSON.stringify([options.tree, options.versions ?? {}, packageJson]);
        if (this.#transformed?.inputs !== inputs) {
            const transformer = new ModuleTransformer(
                this.environment.files,
                options.versions ?? {},
            );
            this.#transformed = { inputs, transformer };
        }

        return this.#transformed.transformer;
    }

    /** Close both native compilers. */
    async [Symbol.asyncDispose](): Promise<void> {
        // await both compiler shutdowns
        const results = await Promise.allSettled([
            this.#typescript?.[Symbol.asyncDispose](),
            this.runtimes[Symbol.asyncDispose](),
        ]);

        // report every compiler shutdown failure
        const failures = results
            .filter((result) => result.status === "rejected")
            .map((result): unknown => result.reason);
        if (failures.length) {
            throw new AggregateError(failures, "compiler shutdown failed");
        }
    }

    /** Open the package with an output's entries and its kind's extra modules. */
    #project(options: RequestedBuild, output: PlannedOutput): Promise<PackageSource> {
        return PackageSource.open(
            {
                directory: options.directory,
                ...(options.configuration === undefined
                    ? {}
                    : { configuration: options.configuration }),
                runtime: output.options.runtime,
                ...(output.options.entries === undefined
                    ? {}
                    : { entries: output.options.entries }),
                files: output.files,
                ...(options.versions === undefined ? {} : { versions: options.versions }),
                tree: options.tree,
            },
            this.environment,
        );
    }

    /** Pick the output whose program reads the whole source, by the runtime declarations run on. */
    static #primary(planned: ReadonlyMap<string, PlannedOutput>): string {
        // prefer Bun, where declarations evaluate, then workerd, then the browser
        const outputs = [...planned].toSorted(
            ([, left], [, right]) =>
                PRIMARY_RUNTIMES.indexOf(left.options.runtime) -
                PRIMARY_RUNTIMES.indexOf(right.options.runtime),
        );

        return present(outputs[0], "a planned output")[0];
    }

    /** Compile each output once, a kind's outputs in one pass, joining their dependencies. */
    async #compile(
        options: RequestedBuild,
        planned: ReadonlyMap<string, PlannedOutput>,
        projects: ReadonlyMap<string, PackageSource>,
        declarations: readonly EvaluatedDeclaration[],
        extensions: readonly BuildExtension[],
        files: BuildDirectory,
        cache: TransformCache,
        transformer: ModuleTransformer,
    ): Promise<{
        outputs: Map<string, Pick<OutputBuild, "output" | "inspection" | "npm">>;
        sourceMaps: SourceMapReference[];
        resolved: Record<string, DependencyResolution>;
    }> {
        // compile each request once, all at once
        const context = { options, projects, declarations, extensions, files, cache, transformer };
        const started = new Map<
            string,
            Promise<{
                outputs: Record<string, Pick<OutputBuild, "output" | "inspection" | "npm">>;
                sourceMaps: SourceMapReference[];
            }>
        >();
        for (const [name, output] of planned) {
            const request = output.pass?.name ?? name;
            if (!started.has(request)) {
                started.set(
                    request,
                    output.pass === undefined
                        ? this.#module(name, output, context)
                        : this.#pass(output.pass, planned, context),
                );
            }
        }
        const requests = new Map(
            await Promise.all(
                [...started].map(async ([request, compiled]) => [request, await compiled] as const),
            ),
        );

        // collect each output's description, the dependencies they resolve and every source map
        const outputs = new Map<string, Pick<OutputBuild, "output" | "inspection" | "npm">>();
        const resolved: Record<string, DependencyResolution> = {};
        for (const [name, output] of planned) {
            const described = found(requests, output.pass?.name ?? name).outputs[name];
            if (described === undefined) {
                throw BuildError.of("INVARIANT_VIOLATED", `output ${name} has no compilation`);
            }
            outputs.set(name, described);
            for (const [key, dependency] of Object.entries(described.inspection.packages)) {
                resolved[key] = joinResolutions(key, resolved[key], dependency);
            }
        }
        const sourceMaps = [...requests.values()].flatMap((entry) => entry.sourceMaps);

        return { outputs, sourceMaps, resolved };
    }

    /** Compile a module output with the extensions' plugins and write its files. */
    async #module(
        name: string,
        output: PlannedOutput,
        context: CompileContext,
    ): Promise<{
        outputs: Record<string, Pick<OutputBuild, "output" | "inspection" | "npm">>;
        sourceMaps: SourceMapReference[];
    }> {
        // compile the output and write its files
        const compilation = createCompilation(name, output, context);
        const compiled = await compilation.compile(
            context.options.dependencies,
            context.files,
            context.files.directory,
        );
        await writeFiles(compiled.files, compiled.paths, context.files);
        checkInputs(compiled.inspection, compilation.runtime);

        return {
            outputs: {
                [name]: {
                    output: compiled.output,
                    inspection: compiled.inspection,
                    npm: compiled.npm,
                },
            },
            sourceMaps: compiled.sourceMaps,
        };
    }

    /** Compile the outputs of one kind's request in a separate pass and write their files. */
    async #pass(
        request: NonNullable<PlannedOutput["pass"]>,
        planned: ReadonlyMap<string, PlannedOutput>,
        context: CompileContext,
    ): Promise<{
        outputs: Record<string, Pick<OutputBuild, "output" | "inspection" | "npm">>;
        sourceMaps: SourceMapReference[];
    }> {
        // lend the kind the build's inputs for the outputs its request expanded into
        const plans = [...planned].filter(([, plan]) => plan.pass?.name === request.name);
        const members = plans.map(([member]) => member);
        const compilations = Object.fromEntries(
            plans.map(([member, plan]) => [member, createCompilation(member, plan, context)]),
        );
        const pass = new OutputPass(
            found(context.projects, nth(members, 0)),
            context.options.dependencies,
            context.files,
            context.files.directory,
            compilations,
            context.extensions,
            context.cache,
            context.transformer,
        );

        // compile the outputs and write their files
        let described: Readonly<Record<string, PackageOutput>>;
        try {
            described = await request.kind.compile(
                request.name,
                request.request,
                compilations,
                pass,
            );
        } catch (cause) {
            throw extensionFailureOf(cause);
        }
        await writeFiles(pass.files, pass.paths, context.files);

        // describe each output by the kind's manifest output and the pass's module graph
        const outputs: Record<string, Pick<OutputBuild, "output" | "inspection" | "npm">> = {};
        for (const member of members) {
            const output = described[member];
            if (output === undefined) {
                throw BuildError.of(
                    "INVARIANT_VIOLATED",
                    `the pass did not describe output ${member}`,
                );
            }
            outputs[member] = { output, inspection: pass.inspection(member), npm: new Set() };
        }

        return { outputs, sourceMaps: pass.sourceMaps };
    }

    /** Run one request, answering its result or its failure. */
    static async #respond(
        compiler: CompilerWorker,
        request: BuildRequest,
        cache: TransformCache,
    ): Promise<BuildResponse> {
        try {
            // build the package
            if (request.kind === "build") {
                const npm = await compiler.build(
                    request.options,
                    request.declarations,
                    request.destination,
                    cache,
                );

                return { kind: "result", result: { kind: "build", npm } };
            }
            // evaluate a package's declarations
            else if (request.kind === "evaluate") {
                const declarations = await compiler.evaluate(request.options);

                return { kind: "result", result: { kind: "evaluate", declarations } };
            }
            // check a package's outputs
            else if (request.kind === "check") {
                await compiler.check(request.options);

                return { kind: "result", result: { kind: "check" } };
            }
            // index the package's code
            else if (request.kind === "index") {
                const { code, modules } = await compiler.index(request.options);

                return { kind: "result", result: { kind: "index", code, modules } };
            }
            // inspect the package
            else {
                const inspection = await compiler.inspect(request.options);

                return { kind: "result", result: { kind: "inspect", inspection } };
            }
        } catch (error) {
            return { kind: "result", error: describeFailure(error) };
        }
    }
}

/** Read a source's package.json as released: as authored, or at the release's version. */
async function releasedManifest(project: PackageSource): Promise<Uint8Array<ArrayBuffer>> {
    // take the authored bytes of the version they list
    const path = resolve(project.directory, "package.json");
    const authored = await project.environment.files.readTextFile(path);
    const manifest = schema.jsonObject().parse(JSON.parse(authored));
    const { version } = project.packageJson.package;
    if (manifest["version"] === version) {
        return new TextEncoder().encode(authored);
    }

    return new TextEncoder().encode(`${JSON.stringify({ ...manifest, version }, null, 4)}\n`);
}

/** Expand the requested outputs into module outputs and the outputs of each kind's pass. */
function expand(
    requests: PackageBuildOptions["outputs"],
    extensions: readonly BuildExtension[],
): Map<string, PlannedOutput> {
    // plan each requested output
    const planned = new Map<string, PlannedOutput>();
    for (const [name, request] of Object.entries(requests)) {
        // plan a module output
        if (isModuleRequest(request)) {
            addOutput(planned, name, { options: request, files: [] });
        }
        // plan the outputs a kind expands its request into
        else {
            const kind = kindOf(request.kind, extensions);
            let expanded: Readonly<Record<string, ExpandedOutput>>;
            try {
                expanded = kind.expand(name, request);
            } catch (cause) {
                throw extensionFailureOf(cause);
            }
            for (const [member, output] of Object.entries(expanded)) {
                const { files, ...settings } = output;
                addOutput(planned, member, {
                    options: { kind: "module", ...settings },
                    files,
                    pass: { name, kind, request },
                });
            }
        }
    }

    // require at least one output
    if (!planned.size) {
        throw BuildError.of("OUTPUT_INVALID", "a build requires at least one output");
    }

    return planned;
}

/** Plan an output, refusing an invalid or taken name. */
function addOutput(planned: Map<string, PlannedOutput>, name: string, output: PlannedOutput): void {
    if (!/^[a-z][a-z0-9-]*$/u.test(name)) {
        throw BuildError.of("OUTPUT_INVALID", `invalid output name: ${name}`);
    }
    if (planned.has(name)) {
        throw BuildError.of("OUTPUT_INVALID", `duplicate output name: ${name}`);
    }
    planned.set(name, output);
}

/** Report whether an output request compiles package modules rather than a kind's output. */
function isModuleRequest(request: ModuleOptions | OutputRequest): request is ModuleOptions {
    return request.kind === "module";
}

/** Find the extension compiling an output kind. */
function kindOf(name: string, extensions: readonly BuildExtension[]): OutputKind {
    const kind = extensions.find((extension) => extension.outputs?.[name])?.outputs?.[name];
    if (kind === undefined) {
        throw BuildError.of("OUTPUT_INVALID", `no dependency compiles outputs of kind ${name}`);
    }

    return kind;
}

/** Create the compilation of one planned output. */
function createCompilation(
    name: string,
    output: PlannedOutput,
    context: CompileContext,
): OutputCompilation {
    return new OutputCompilation(
        name,
        output.options,
        found(context.projects, name),
        context.declarations,
        context.extensions,
        output.pass !== undefined,
        context.cache,
        context.transformer,
    );
}

/** Retain the package's template assets, manifests and catalogs before compiling. */
async function retain(project: PackageSource, files: BuildDirectory): Promise<void> {
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

    // retain the package's catalogs
    const owner = project.packageJson.package.id;
    for (const [path, bytes] of await readCatalogs(project.directory, owner, system)) {
        files.retain(path, bytes);
    }
}

/** Write a compilation's generated files, and record the files its compiler wrote. */
async function writeFiles(
    generated: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
    paths: readonly string[],
    files: BuildDirectory,
): Promise<void> {
    // write the generated files
    for (const [path, bytes] of generated) {
        await files.write(path, bytes);
    }

    // record the files the compiler wrote
    for (const path of paths) {
        await files.record(path);
    }
}

/** Join two outputs' resolutions of a dependency, uniting the source package files they read. */
function joinResolutions(
    key: string,
    previous: DependencyResolution | undefined,
    next: DependencyResolution,
): DependencyResolution {
    // take the first resolution and accept an equal one
    if (previous === undefined || JSON.stringify(previous) === JSON.stringify(next)) {
        return next;
    }

    // refuse different releases or packages
    if (
        previous.kind !== "source" ||
        next.kind !== "source" ||
        JSON.stringify(previous.package) !== JSON.stringify(next.package)
    ) {
        throw BuildError.of("DEPENDENCY_INVALID", `conflicting dependency resolution: ${key}`);
    }

    // unite the files by path, refusing one path read with different bytes
    const files = new Map(previous.files.map((file) => [file.path, file]));
    for (const file of next.files) {
        const known = files.get(file.path);
        if (known !== undefined && known.digest !== file.digest) {
            throw BuildError.of(
                "DEPENDENCY_INVALID",
                `conflicting dependency file: ${key}/${file.path}`,
            );
        }
        files.set(file.path, file);
    }
    const ordered = [...files.values()].toSorted(comparePath);

    return { ...next, files: ordered };
}

/** Read the identity of a Destack package among the build tool's dependencies. */
async function findPackage(
    name: string,
    locator: PackageLocator,
    environment: Environment<"process">,
): Promise<Package> {
    // find the package among the tools, refusing a missing one
    const directory = Toolchain.locate([name], PACKAGE, environment.process);
    const owner = await locator.find(join(directory, "package.json"));
    if (!owner) {
        throw BuildError.of("INVARIANT_VIOLATED", `missing Destack package: ${name}`);
    }

    return owner.metadata.package;
}

/** Describe a failure for the host, with a build failure's reason, its cause and its stack. */
function describeFailure(error: unknown): Extract<BuildResponse, { error: object }>["error"] {
    // describe a thrown value that is no error by its text
    if (!(error instanceof Error)) {
        const failure = BuildError.of("INVARIANT_VIOLATED", String(error));

        return { failure: failure.toCallError(), audience: failure.audience };
    }

    // keep a family's failure with its audience, and report anything else as an internal fault
    const failure = DomainError.is(error)
        ? error
        : BuildError.of("INVARIANT_VIOLATED", error.message);

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
