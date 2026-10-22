import { writeSync } from "node:fs";
import { readFile, realpath } from "node:fs/promises";
import { DependencyTree } from "../dependency/tree.ts";
import type { TestDeclaration } from "../typescript/test.ts";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
    type DependencyResolution,
    graph,
    type Package,
    type DeclarationDescription,
} from "@destack/package";
import type { ModuleDescription } from "@destack/package/code";
import { nth, type Commit, Digest, found, present, schema } from "@destack/schema";
import {
    type BuildExtension,
    type ExpandedOutput,
    type OutputKind,
    type OutputRequest,
    type TransformCache,
} from "@destack/package/build";
import { type BuildDescription, type PackageOutput } from "@destack/package/manifest";
import { type SourceMapReference } from "@destack/package/source";
import { PackageLocator } from "@destack/package/locator";
import { Toolchain } from "@destack/check";
import {
    type ExampleDeclaration,
    type ScenarioDeclaration,
    type TypeScriptInspection,
    TypeScriptCompiler,
} from "../typescript/index.ts";
import { inspectModules, type InspectOptions } from "../inspection/inspection.ts";
import { Upgrade, type History } from "@destack/resource";
import { Evaluation, kindKey, selectDeclarations } from "../declaration/index.ts";
import { PackageSource, readPackageJson } from "../source/index.ts";
import { OutputCompilation, type CompiledFiles } from "../compile/compilation.ts";
import { type LoadedExtension, loadExtensions } from "../compile/extension.ts";
import {
    type OutputKeys,
    type CachedOutput,
    deriveModuleKeys,
    Identities,
    identifyToolchain,
    importClosure,
} from "../cache/index.ts";
import { NpmConversion } from "../compile/conversion.ts";
import { OutputPass } from "../compile/pass.ts";
import { checkRuntime, RuntimeCompiler } from "../compile/runtime.ts";
import { Template } from "../template/index.ts";
import { BuildError, isMissing } from "../error/index.ts";
import { isAuthored, relativePath } from "../source/dependency.ts";
import { ProgramImports } from "../cache/scan.ts";
import type { PackageBuildOptions, ModuleOptions } from "./build.ts";
import { BuildDirectory } from "./directory.ts";
import { readCatalogs } from "./catalog.ts";
import { comparePath, stringifyInspection } from "./serialization.ts";
import { describeCode, describeGraph, joinModules } from "../graph/module.ts";
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
import { PRIMARY_RUNTIMES } from "@destack/package/runtime";

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

/** A program's imports and the keys of its modules. */
interface ProgramKeys {
    /** The program's imports. */
    readonly imports: ProgramImports;
    /** The key of each of the package's modules, by absolute path. */
    readonly keys: ReadonlyMap<string, Digest>;
}

/** An output's opened package source and inspection. */
interface InspectedOutput {
    /** The opened package source. */
    readonly project: PackageSource;
    /** The compiler's inspection. */
    readonly inspection: TypeScriptInspection;
    /** The evaluated declarations of the package and its imported dependencies. */
    readonly evaluation: Evaluation;
}

/** The compiler inside the sandbox, retaining the last package's state across requests. */
export class CompilerWorker implements AsyncDisposable {
    /** The native TypeScript compiler of the last requested package. */
    #typescript: TypeScriptCompiler | undefined;
    /** Runtime type environments reused across output checks. */
    readonly runtimes = new RuntimeCompiler();
    /** The Destack packages the build tool's dependencies resolve to. */
    readonly #packages = new PackageLocator();
    /** Configurations indexed by output settings. */
    readonly #sources = new Map<string, { declaration: string; source: PackageSource }>();
    /** The identity of the toolchain this process runs, read once. */
    #toolchain: Promise<Digest> | undefined;
    /** The identity of each loaded extension's package, read once as the process caches code. */
    readonly #extensions = new Map<string, Promise<Digest>>();

    /** Switch to a package's native compiler, closing the others' compiler and configurations. */
    async #switch(directory: string): Promise<TypeScriptCompiler> {
        // reuse the compiler of the package the last request listed
        const current = this.#typescript;
        if (current?.directory === directory) {
            return current;
        }

        // close the last package's compiler and the other packages' configurations
        await current?.[Symbol.asyncDispose]();
        for (const [key, { source }] of this.#sources) {
            if (source.directory !== directory) {
                await source[Symbol.asyncDispose]();
                this.#sources.delete(key);
            }
        }

        // start the package's compiler
        this.#typescript = new TypeScriptCompiler(directory);

        return this.#typescript;
    }

    /** Answer the host's requests on stdin with one retained compiler. */
    static async run(): Promise<void> {
        // reserve stdout for framed build results and send package logs to diagnostics
        Object.assign(
            globalThis.console,
            new Console({ stdout: process.stderr, stderr: process.stderr }),
            { write: writeDiagnostics },
        );
        const cache = new HostCache(process.stdout);
        await using compiler = new CompilerWorker();

        // read the host's messages while requests run
        let responding = Promise.resolve();
        for await (const message of readMessages(process.stdin, HostMessage)) {
            // deliver the host's cache answers at once
            if (message.kind === "cached") {
                cache.answer(message);
            }
            // answer the host's requests one at a time
            else {
                responding = responding.then(async () =>
                    writeMessage(
                        process.stdout,
                        await CompilerWorker.#respond(compiler, message, cache),
                    ),
                );
            }
        }

        // fail the cache requests the host can no longer answer, then finish the requests
        cache.close(new BuildError("BUILD_FAILED", "the host closed the compiler's input"));
        await responding;
    }

    /** Inspect source using retained compiler state and source configurations. */
    async inspect(options: InspectOptions & Pick<RequestedBuild, "tree">) {
        const { project, inspection, evaluation } = await this.#inspectPackage(options);

        return inspectModules(
            project.packageJson.package,
            inspection.modules,
            inspection.tests,
            evaluation.declarations,
        );
    }

    /** Index source completely: every module's symbols and the edges of its code. */
    async index(options: InspectOptions & Pick<RequestedBuild, "tree">): Promise<graph.Document[]> {
        const { project, inspection, evaluation } = await this.#inspectPackage(options);

        return describeCode(
            project.packageJson.package,
            inspection.modules,
            evaluation.declarations,
            {
                package: await findPackage("@destack/test", this.#packages),
                declarations: inspection.tests,
            },
        );
    }

    /** Inspect the whole package with its dependencies' builds and evaluate its declarations. */
    async #inspectPackage(
        options: InspectOptions & Pick<RequestedBuild, "tree">,
    ): Promise<InspectedOutput> {
        // inspect source with the selected compiler configuration and the dependencies' builds
        const project = await this.open({ ...options, roots: "package" });
        const typescript = await this.#switch(options.directory);
        const [inspected] = await typescript.inspect([project], "index");
        const inspection = present(inspected, "the inspection");

        // evaluate the collected domain declarations
        const evaluation = await Evaluation.evaluate(inspection.declarations, [], project);

        return { project, inspection, evaluation };
    }

    /** Identify an output's non-module inputs: toolchain, extensions and manifests. */
    async #identifyInputs(
        loaded: Awaited<ReturnType<typeof loadExtensions>>,
        directory: string,
        identities: Identities,
    ) {
        this.#toolchain ??= identifyToolchain(Identities.of({}));
        const files = (paths: Iterable<string>) =>
            Promise.all([...paths].map((path) => identities.file(join(directory, path))));

        return {
            toolchain: await this.#toolchain,
            extensions: await Promise.all(loaded.map((entry) => this.#identify(entry, identities))),
            manifests: await files(["package.json", "destack.json"]),
        };
    }

    /** Derive each requested output's cache key without checking, evaluating or compiling it. */
    async plan(options: RequestedBuild): Promise<OutputKeys> {
        // plan the outputs with the extensions of the package's dependency closure
        const packageJson = await readPackageJson(options.directory);
        const tree = await DependencyTree.read(options.tree);
        const loaded = await loadExtensions(options.directory, packageJson, tree);
        const planned = expand(
            options.outputs,
            loaded.map((entry) => entry.extension),
        );

        // identify the toolchain, the extensions and the package's manifests
        const identities = Identities.of(options.dependencies);
        const { toolchain, extensions, manifests } = await this.#identifyInputs(
            loaded,
            options.directory,
            identities,
        );

        // key each requested output by its request, toolchain, dependencies and the modules it imports
        const outputs = await this.#keyModules(options, planned, toolchain, identities);
        const keys: Record<string, Digest> = {};
        for (const [request, imported] of outputs) {
            keys[request] = await Digest.json({
                toolchain,
                extensions,
                manifests,
                dependencies: options.dependencies,
                versions: options.versions ?? {},
                request: present(options.outputs[request], `the requested output ${request}`),
                configuration: options.configuration ?? null,
                modules: [...imported].toSorted(),
            });
        }

        return keys;
    }

    /** Build a source package into a directory, answering the outputs it compiled. */
    async build(
        options: RequestedBuild,
        reuse: Readonly<Record<string, CachedOutput>>,
        destination: string,
        cache: TransformCache,
    ): Promise<Record<string, CachedOutput>> {
        // plan the outputs with the extensions of the package's dependency closure
        const packageJson = await readPackageJson(options.directory);
        const tree = await DependencyTree.read(options.tree);
        const extensions = (await loadExtensions(options.directory, packageJson, tree)).map(
            (entry) => entry.extension,
        );
        const planned = expand(options.outputs, extensions);

        // inspect, check, evaluate and retain every output's sources before compiling any
        const inspected = await this.#inspect(options, planned);
        const files = new BuildDirectory(destination);
        await retain(inspected, files);

        // compile the outputs the cache lacks
        const compiled = await this.#compile(
            options,
            planned,
            inspected,
            extensions,
            files,
            reuse,
            cache,
        );

        // describe the declarations each output selects
        const description = await this.#describe(planned, inspected, compiled);

        // plan the upgrade from what the package has published
        const source = sourcePackage(inspected);
        const upgrade =
            options.history === undefined
                ? undefined
                : planUpgrade(options.history, source, description.graph, inspected);

        // write the manifest
        await writeManifest(
            source,
            options.commit,
            compiled,
            description,
            upgrade,
            files,
            this.#packages,
        );

        return compiled.cached;
    }

    /** Reuse a configuration until its package declarations change. */
    async open(options: Parameters<typeof PackageSource.open>[0]): Promise<PackageSource> {
        // read the manifests on every build so changed exports update the compiler roots
        const manifests = await Promise.all(
            ["package.json", "destack.json"].map((path) =>
                readFile(join(options.directory, path), "utf8"),
            ),
        );

        // include authored compiler settings in configuration invalidation
        const configuration = await readOptional(
            resolve(options.directory, options.configuration ?? "tsconfig.json"),
        );

        // reuse the configuration while its inputs remain unchanged
        const declaration = JSON.stringify([manifests, configuration]);
        const key = JSON.stringify(options);
        const previous = this.#sources.get(key);
        if (previous?.declaration === declaration) {
            return previous.source;
        }

        // replace changed configurations and retain the new source until shutdown
        const source = await PackageSource.open(options);
        await previous?.source[Symbol.asyncDispose]();
        this.#sources.set(key, { declaration, source });

        return source;
    }

    /** Close the native compiler before releasing its configurations. */
    async [Symbol.asyncDispose](): Promise<void> {
        try {
            // await both compiler shutdowns before releasing source configurations
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
        } finally {
            await Promise.all(
                [...this.#sources.values()].map(({ source }) => source[Symbol.asyncDispose]()),
            );
        }
    }

    /** Open the package with an output's entries, its kind's extra modules and its declarations. */
    #project(
        options: RequestedBuild,
        output: PlannedOutput,
        roots: "package" | "entries",
        declarations: readonly string[],
    ): Promise<PackageSource> {
        return this.open({
            roots,
            directory: options.directory,
            ...(options.configuration === undefined
                ? {}
                : { configuration: options.configuration }),
            runtime: output.options.runtime,
            ...(output.options.entries === undefined ? {} : { entries: output.options.entries }),
            files: roots === "entries" ? [...output.files, ...declarations] : output.files,
            ...(options.versions === undefined ? {} : { versions: options.versions }),
            tree: options.tree,
        });
    }

    /** List the package's ambient declaration files, which every entry-rooted program reads. */
    async #declarations(options: RequestedBuild): Promise<string[]> {
        // read the authored configuration's root files, none without one
        const typescript = await this.#switch(options.directory);
        const configured = await typescript
            .configure(options.configuration ?? "tsconfig.json")
            .catch((error: unknown) => {
                if (!isMissing(error)) {
                    throw error;
                }
            });

        // take its declaration files
        const directory = await realpath(options.directory);

        return (configured?.files ?? [])
            .filter((file) => file.endsWith(".d.ts") && isAuthored(directory, file))
            .map((file) => relativePath(directory, file));
    }

    /** Key each distinct program's modules once, collecting each output's module keys. */
    async #keyModules(
        options: RequestedBuild,
        planned: ReadonlyMap<string, PlannedOutput>,
        toolchain: Digest,
        identities: Identities,
    ): Promise<Map<string, Set<Digest>>> {
        // open each output's project, gathering the entries each program compiles
        const projects = new Map<string, { project: PackageSource; entries: string[] }>();
        const compiled = new Map<string, string[]>();
        const primary = CompilerWorker.#primary(planned);
        const declarations = await this.#declarations(options);
        for (const [name, output] of planned) {
            const project = await this.#project(
                options,
                output,
                name === primary ? "package" : "entries",
                declarations,
            );
            const entries = await Promise.all(project.entries.map((entry) => realpath(entry)));
            projects.set(name, { project, entries });
            compiled.set(project.configuration, [
                ...(compiled.get(project.configuration) ?? []),
                ...entries,
            ]);
        }

        // collect the keys of each output's modules
        const programs = new Map<string, Promise<ProgramKeys>>();
        const outputs = new Map<string, Set<Digest>>();
        for (const [name, output] of planned) {
            // key each program's modules once
            const { project, entries } = found(projects, name);
            let program = programs.get(project.configuration);
            if (program === undefined) {
                program = this.#keys(
                    project,
                    toolchain,
                    identities,
                    found(compiled, project.configuration),
                );
                programs.set(project.configuration, program);
            }
            const { imports, keys } = await program;

            // collect the keys of the modules each requested output imports
            const request = output.pass?.name ?? name;
            const imported = outputs.get(request) ?? new Set();
            for (const file of importClosure(imports, entries)) {
                imported.add(found(keys, file));
            }
            outputs.set(request, imported);
        }

        return outputs;
    }

    /** Read a program's imports and key its modules. */
    async #keys(
        project: PackageSource,
        toolchain: Digest,
        identities: Identities,
        entries: readonly string[],
    ): Promise<ProgramKeys> {
        // scan the configuration's root files and key the entries' closure
        const typescript = await this.#switch(project.directory);
        const configuration = await typescript.configure(project.configuration);
        const imports = await ProgramImports.scan(project.directory, configuration, identities);
        const compiled = importClosure(imports, entries);
        const keys = await deriveModuleKeys(
            imports,
            project.directory,
            toolchain,
            identities,
            compiled,
        );

        return { imports, keys };
    }

    /** Identify an extension's package once, as the process caches the first imported extension. */
    #identify(loaded: LoadedExtension, identities: Identities): Promise<Digest> {
        let identity = this.#extensions.get(loaded.directory);
        if (identity === undefined) {
            identity = identities.package(loaded.directory);
            this.#extensions.set(loaded.directory, identity);
        }

        return identity;
    }

    /** Open and inspect each output, check every source, then evaluate each inspection once. */
    async #inspect(
        options: RequestedBuild,
        planned: ReadonlyMap<string, PlannedOutput>,
    ): Promise<Map<string, InspectedOutput>> {
        // open each output's project, the primary one reading the whole source
        const primary = CompilerWorker.#primary(planned);
        const ordered = [...planned].toSorted(
            ([left], [right]) => Number(right === primary) - Number(left === primary),
        );
        const projects = new Map<string, PackageSource>();
        const declarations = await this.#declarations(options);
        let source: Package | undefined;
        for (const [name, output] of ordered) {
            const project = await this.#project(
                options,
                output,
                name === primary ? "package" : "entries",
                declarations,
            );
            const identity = project.packageJson.package;
            if (source !== undefined && !isSamePackage(source, identity)) {
                throw new BuildError("BUILD_FAILED", "package changed during build");
            }
            source = identity;
            projects.set(name, project);
        }

        // inspect each distinct program together, the primary one first
        const programs = [
            ...new Map(
                [...projects.values()].map((project) => [project.configuration, project]),
            ).values(),
        ];
        const typescript = await this.#switch(options.directory);
        const inspections = await typescript.inspect(programs, "build");
        const opened = new Map<string, Omit<InspectedOutput, "evaluation">>();
        for (const [name, project] of projects) {
            const index = programs.findIndex(
                (each) => each.configuration === project.configuration,
            );
            opened.set(name, { project, inspection: nth(inspections, index) });
        }

        // evaluate each distinct inspection's declarations once
        const evaluated = new Map<TypeScriptInspection, Evaluation>();
        const inspected = new Map<string, InspectedOutput>();
        for (const name of planned.keys()) {
            const { project, inspection } = found(opened, name);
            let evaluation = evaluated.get(inspection);
            if (!evaluation) {
                evaluation = await Evaluation.evaluate(
                    inspection.declarations,
                    inspection.examples,
                    project,
                );
                evaluated.set(inspection, evaluation);
            }
            inspected.set(name, { project, inspection, evaluation });
        }

        return inspected;
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

    /** Compile each uncached output, a kind's outputs in one pass, and reuse the others. */
    async #compile(
        options: RequestedBuild,
        planned: ReadonlyMap<string, PlannedOutput>,
        inspected: ReadonlyMap<string, InspectedOutput>,
        extensions: readonly BuildExtension[],
        files: BuildDirectory,
        reuse: Readonly<Record<string, CachedOutput>>,
        cache: TransformCache,
    ): Promise<{
        outputs: Map<string, Pick<CompiledFiles, "output" | "inspection">>;
        sourceMaps: SourceMapReference[];
        cached: Record<string, CachedOutput>;
    }> {
        // record the reused files the caller wrote before any compilation writes its files
        await recordReused(reuse, files);

        // compile each request the cache lacks once, in planned order
        const requests = new Map<string, CachedOutput>();
        const cached: Record<string, CachedOutput> = {};
        for (const [name, output] of planned) {
            const request = output.pass?.name ?? name;
            const reused = Object.hasOwn(reuse, request) ? reuse[request] : undefined;

            // return a request compiled or reused before
            if (requests.has(request)) {
                continue;
            }
            // reuse a cached request
            else if (reused !== undefined) {
                requests.set(request, reused);
            }
            // compile a module output, or the outputs of a kind's pass
            else {
                const compiled =
                    output.pass === undefined
                        ? await this.#module(
                              name,
                              output,
                              options,
                              inspected,
                              extensions,
                              files,
                              cache,
                          )
                        : await this.#pass(
                              output.pass,
                              options,
                              planned,
                              inspected,
                              extensions,
                              files,
                              cache,
                          );
                requests.set(request, compiled);
                cached[request] = compiled;
            }
        }

        // collect each output's description and every source map
        const outputs = new Map<string, Pick<CompiledFiles, "output" | "inspection">>();
        for (const [name, output] of planned) {
            const described = found(requests, output.pass?.name ?? name).outputs[name];
            if (described === undefined) {
                throw new BuildError("BUILD_FAILED", `no compilation described output ${name}`);
            }
            outputs.set(name, { output: described.output, inspection: described.description });
        }
        const sourceMaps = [...requests.values()].flatMap((entry) => entry.sourceMaps);

        return { outputs, sourceMaps, cached };
    }

    /** Compile a module output with the extensions' plugins and write its files. */
    async #module(
        name: string,
        output: PlannedOutput,
        options: RequestedBuild,
        inspected: ReadonlyMap<string, InspectedOutput>,
        extensions: readonly BuildExtension[],
        files: BuildDirectory,
        cache: TransformCache,
    ): Promise<CachedOutput> {
        // compile the output and write its files
        const compilation = createCompilation(name, output, inspected, extensions, cache);
        const compiled = await compilation.compile(
            options.dependencies,
            files.retained,
            files.directory,
        );
        await writeFiles(compiled.files, compiled.paths, files);

        return {
            outputs: { [name]: { output: compiled.output, description: compiled.inspection } },
            sourceMaps: compiled.sourceMaps,
            files: [...compiled.files.keys(), ...compiled.paths].map((path) => files.file(path)),
        };
    }

    /** Compile the outputs of one kind's request in a separate pass and write their files. */
    async #pass(
        request: NonNullable<PlannedOutput["pass"]>,
        options: RequestedBuild,
        planned: ReadonlyMap<string, PlannedOutput>,
        inspected: ReadonlyMap<string, InspectedOutput>,
        extensions: readonly BuildExtension[],
        files: BuildDirectory,
        cache: TransformCache,
    ): Promise<CachedOutput> {
        // lend the kind the build's inputs for the outputs its request expanded into
        const plans = [...planned].filter(([, plan]) => plan.pass?.name === request.name);
        const members = plans.map(([member]) => member);
        const compilations = Object.fromEntries(
            plans.map(([member, plan]) => [
                member,
                createCompilation(member, plan, inspected, extensions, cache),
            ]),
        );
        const pass = new OutputPass(
            found(inspected, nth(members, 0)).project,
            options.dependencies,
            files.retained,
            files.directory,
            compilations,
            this.runtimes,
            extensions,
            cache,
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
            throw BuildError.from(cause);
        }
        await writeFiles(pass.files, pass.paths, files);

        // describe each output by the kind's manifest output and the pass's module graph
        const outputs: CachedOutput["outputs"] = {};
        for (const member of members) {
            const output = described[member];
            if (output === undefined) {
                throw new BuildError("BUILD_FAILED", `the pass described no output ${member}`);
            }
            outputs[member] = { output, description: pass.inspection(member) };
        }

        return {
            outputs,
            sourceMaps: pass.sourceMaps,
            files: [...pass.files.keys(), ...pass.paths].map((path) => files.file(path)),
        };
    }

    /** Check each module output against its runtime, and collect the declarations each selects. */
    async #describe(
        planned: ReadonlyMap<string, PlannedOutput>,
        inspected: ReadonlyMap<string, InspectedOutput>,
        compiled: { outputs: ReadonlyMap<string, Pick<CompiledFiles, "output" | "inspection">> },
    ): Promise<{ graph: graph.Module[]; resolved: Record<string, DependencyResolution> }> {
        // share equal tests across outputs
        const modules = new Map<string, ModuleDescription>();
        const evaluated = new Set<DeclarationDescription>();
        const tests = new SharedValues<TestDeclaration>();
        const examples = new SharedValues<ExampleDeclaration>();
        const scenarios = new SharedValues<ScenarioDeclaration>();
        const resolved: Record<string, DependencyResolution> = {};
        for (const [name, { output, inspection: description }] of compiled.outputs) {
            // check the output's dependency declarations and runtime
            const entry = found(inspected, name);
            await this.#checkOutput(entry, found(planned, name), output, description);

            // retain exact dependency resolutions once, joining the files outputs read of a source
            for (const [key, dependency] of Object.entries(description.packages)) {
                resolved[key] = joinResolutions(key, resolved[key], dependency);
            }

            // join each module's descriptions and collect every evaluated declaration for the graph
            retainGraphInputs(entry, modules, evaluated);

            // list each test, example and scenario once across outputs
            for (const test of entry.inspection.tests) {
                tests.retain(test);
            }
            for (const example of entry.evaluation.examples) {
                examples.retain(example);
            }
            for (const scenario of entry.inspection.scenarios) {
                scenarios.retain(scenario);
            }
        }

        // describe the package's graph from its modules and evaluated declarations
        const described = describeGraph(
            sourcePackage(inspected),
            [...modules.values()],
            [...evaluated],
            {
                package: await findPackage("@destack/test", this.#packages),
                declarations: tests.values,
            },
            {
                package: await findPackage("@destack/package", this.#packages),
                examples: examples.values,
                scenarios: scenarios.values,
            },
        );

        return { graph: described, resolved };
    }

    /** Require an output's dependency declarations at resolved versions and check its runtime. */
    async #checkOutput(
        inspected: InspectedOutput,
        planned: PlannedOutput,
        output: PackageOutput,
        description: BuildDescription,
    ): Promise<void> {
        // require every dependency declaration at its exact resolved version
        const { project, inspection, evaluation } = inspected;
        const source = project.packageJson.package;
        for (const declaration of selectDeclarations(
            evaluation.declarations,
            source,
            description,
        )) {
            requirePresent(declaration, source, description, output);
        }

        // check module outputs against their runtime, leaving kinds to check theirs
        if (planned.pass === undefined) {
            await checkRuntime(description, inspection.modules, project.runtime, this.runtimes);
        }
    }

    /** Run one request, answering its result or its failure. */
    static async #respond(
        compiler: CompilerWorker,
        request: BuildRequest,
        cache: TransformCache,
    ): Promise<BuildResponse> {
        try {
            // plan a build's keys
            if (request.kind === "plan") {
                const keys = await compiler.plan(request.options);

                return { kind: "result", result: { kind: "plan", keys } };
            }
            // build the package, reusing the cached outputs
            else if (request.kind === "build") {
                const outputs = await compiler.build(
                    request.options,
                    request.reuse,
                    request.destination,
                    cache,
                );

                return {
                    kind: "result",
                    result: { kind: "build", outputs },
                };
            }
            // convert an npm package for workerd
            else if (request.kind === "convert") {
                const conversion = new NpmConversion(
                    request.name,
                    request.dependencies,
                    request.peers,
                );
                const imported = await conversion.convert(request.source, request.destination);

                return { kind: "result", result: { kind: "convert", imported } };
            }
            // index the package's code
            else if (request.kind === "index") {
                const code = await compiler.index(request.options);

                return { kind: "result", result: { kind: "index", code } };
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

/** Join each module's descriptions across outputs and collect each output's declarations. */
function retainGraphInputs(
    inspected: InspectedOutput,
    modules: Map<string, ModuleDescription>,
    evaluated: Set<DeclarationDescription>,
): void {
    // join a module each runtime inspects into one description
    for (const module of inspected.inspection.modules) {
        const previous = modules.get(module.path);
        modules.set(module.path, previous === undefined ? module : joinModules(previous, module));
    }
    for (const declaration of inspected.evaluation.declarations) {
        evaluated.add(declaration);
    }
}

/** Read the package every inspected output opens. */
function sourcePackage(inspected: ReadonlyMap<string, InspectedOutput>): Package {
    return present(inspected.values().next().value, "an inspected output").project.packageJson
        .package;
}

/** Read a source's package.json as released: as authored, or at the release's version. */
async function releasedManifest(project: PackageSource): Promise<Uint8Array<ArrayBuffer>> {
    // take the authored bytes of the version they list
    const authored = await readFile(resolve(project.directory, "package.json"), "utf8");
    const manifest = schema.jsonObject().parse(JSON.parse(authored));
    const { version } = project.packageJson.package;
    if (manifest["version"] === version) {
        return new TextEncoder().encode(authored);
    }

    return new TextEncoder().encode(`${JSON.stringify({ ...manifest, version }, null, 4)}\n`);
}

/** Report whether two identities are the same package release. */
function isSamePackage(left: Package, right: Package): boolean {
    return left.id === right.id && left.name === right.name && left.version === right.version;
}

/** Read a file's text, absent for a missing file. */
async function readOptional(path: string): Promise<string | undefined> {
    try {
        return await readFile(path, "utf8");
    } catch (error) {
        if (!isMissing(error)) {
            throw error;
        }

        return undefined;
    }
}

/** Record the files of reused outputs the caller wrote, refusing one that changed. */
async function recordReused(
    reuse: Readonly<Record<string, CachedOutput>>,
    files: BuildDirectory,
): Promise<void> {
    for (const entry of Object.values(reuse)) {
        for (const file of entry.files) {
            await files.record(file.path);
            if (files.file(file.path).digest !== file.digest) {
                throw new BuildError("BUILD_FAILED", `a cached file changed: ${file.path}`);
            }
        }
    }
}

/** Values shared by index across outputs, each distinct value once. */
class SharedValues<Value> {
    /** The distinct values, in order of first appearance. */
    readonly values: Value[] = [];
    /** The index of each value, by its serialization. */
    readonly #indices = new Map<string, number>();

    /** Add a value once, returning its index. */
    retain(value: Value): number {
        // reuse the index of an equal value
        const key = stringifyInspection(value);
        const existing = this.#indices.get(key);
        if (existing !== undefined) {
            return existing;
        }

        // add a new value
        const index = this.values.length;
        this.values.push(value);
        this.#indices.set(key, index);

        return index;
    }
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
                throw BuildError.from(cause);
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
        throw new BuildError("BUILD_FAILED", "a build requires at least one output");
    }

    return planned;
}

/** Plan an output, refusing an invalid or taken name. */
function addOutput(planned: Map<string, PlannedOutput>, name: string, output: PlannedOutput): void {
    if (!/^[a-z][a-z0-9-]*$/u.test(name)) {
        throw new BuildError("BUILD_FAILED", `invalid output name: ${name}`);
    }
    if (planned.has(name)) {
        throw new BuildError("BUILD_FAILED", `duplicate output name: ${name}`);
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
        throw new BuildError("BUILD_FAILED", `no dependency compiles outputs of kind ${name}`);
    }

    return kind;
}

/** Create the compilation of one planned output. */
function createCompilation(
    name: string,
    output: PlannedOutput,
    inspected: ReadonlyMap<string, InspectedOutput>,
    extensions: readonly BuildExtension[],
    cache: TransformCache,
): OutputCompilation {
    const { project, inspection, evaluation } = found(inspected, name);

    return new OutputCompilation(
        name,
        output.options,
        project,
        {
            declarations: evaluation.declarations,
            modules: inspection.modules,
            directories: inspection.directories,
        },
        extensions,
        output.pass !== undefined,
        cache,
    );
}

/** Retain every inspected source, template asset, manifest and catalog before compiling. */
async function retain(
    inspected: ReadonlyMap<string, InspectedOutput>,
    files: BuildDirectory,
): Promise<void> {
    for (const { project, inspection } of inspected.values()) {
        // retain inspected sources
        for (const [path, bytes] of inspection.sources) {
            files.retain(path, bytes);
        }

        // retain template assets, leaving package.json to its release's copy below
        if (project.packageJson.definition.template) {
            const template = await Template.read(project.directory);
            for (const [path, bytes] of template.files) {
                if (path !== "package.json") {
                    files.retain(path, new Uint8Array(bytes));
                }
            }
        }

        // retain the authored package declarations, package.json at the version the build releases
        files.retain("package.json", await releasedManifest(project));
        files.retain(
            "destack.json",
            new Uint8Array(await readFile(resolve(project.directory, "destack.json"))),
        );

        // retain the package's catalogs
        const owner = project.packageJson.package.id;
        for (const [path, bytes] of await readCatalogs(project.directory, owner)) {
            files.retain(path, bytes);
        }
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

/** Require a dependency's declaration to come from the dependency at its exact version. */
function requirePresent(
    declaration: DeclarationDescription,
    source: Package,
    description: BuildDescription,
    output: PackageOutput,
): void {
    // accept the package's declarations
    const owner = declaration.symbol.package;
    if (owner.id === source.id && owner.version === source.version) {
        return;
    }

    // require the declaring package at its exact version
    const key = `${owner.name}@${owner.version}`;
    const dependency = description.packages[key] ?? output.dependencies[owner.name];
    if (
        !dependency ||
        dependency.package.version !== owner.version ||
        (dependency.kind !== "npm" && dependency.package.id !== owner.id)
    ) {
        throw new BuildError("BUILD_FAILED", `unresolved declaration package: ${key}`);
    }

    // require the declaring source file in a source dependency
    if (
        dependency.kind === "source" &&
        !dependency.files.some((file) => file.path === declaration.source.file)
    ) {
        throw new BuildError(
            "BUILD_FAILED",
            `missing declaration source: ${key}/${declaration.source.file}`,
        );
    }
}

/** Write the build's sources, graph, lists and upgrade, then the manifest referring to them. */
async function writeManifest(
    source: Package,
    commit: Commit | undefined,
    compiled: {
        outputs: ReadonlyMap<string, Pick<CompiledFiles, "output" | "inspection">>;
        sourceMaps: SourceMapReference[];
    },
    description: { graph: graph.Module[]; resolved: Record<string, DependencyResolution> },
    upgrade: Upgrade | undefined,
    files: BuildDirectory,
    locator: PackageLocator,
): Promise<void> {
    // write the build in its format, the upgrade qualified by the package defining it
    await files.finish({
        package: source,
        ...(commit === undefined ? {} : { commit }),
        outputs: Object.fromEntries(
            [...compiled.outputs].map(([name, { output }]) => [name, output]),
        ),
        dependencies: description.resolved,
        sourceMaps: compiled.sourceMaps,
        graph: description.graph,
        ...(upgrade === undefined
            ? {}
            : {
                  upgrade: {
                      package: await findPackage("@destack/resource", locator),
                      description: upgrade,
                  },
              }),
    });
}

/** Plan the upgrade from what the package has published. */
function planUpgrade(
    history: History,
    source: Package,
    modules: readonly graph.Module[],
    inspected: ReadonlyMap<string, InspectedOutput>,
): Upgrade {
    // compare the package's declarations by their kinds' comparisons
    const compare = new Map(
        [...inspected.values()].flatMap((output) => [...output.evaluation.compare]),
    );
    const declarations = modules
        .flatMap((module) => module.declarations)
        .filter((declaration) => !graph.Declaration.isMember(declaration));

    return Upgrade.plan(history, source, declarations, (declaration) =>
        compare.get(kindKey(declaration)),
    );
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
        throw new BuildError("BUILD_FAILED", `conflicting dependency resolution: ${key}`);
    }

    // unite the files by path, refusing one path read with different bytes
    const files = new Map(previous.files.map((file) => [file.path, file]));
    for (const file of next.files) {
        const known = files.get(file.path);
        if (known !== undefined && known.digest !== file.digest) {
            throw new BuildError(
                "BUILD_FAILED",
                `conflicting dependency file: ${key}/${file.path}`,
            );
        }
        files.set(file.path, file);
    }
    const ordered = [...files.values()].toSorted(comparePath);

    return { ...next, files: ordered };
}

/** Read the identity of a Destack package among the build tool's dependencies. */
async function findPackage(name: string, locator: PackageLocator): Promise<Package> {
    const owner = await locator.find(join(Toolchain.locate([name], PACKAGE), "package.json"));
    if (!owner) {
        throw new BuildError("BUILD_FAILED", `missing Destack package: ${name}`);
    }

    return owner.metadata.package;
}

/** Describe a failure for the host, with a build failure's code, its cause and its stack. */
function describeFailure(error: unknown): Extract<BuildResponse, { error: object }>["error"] {
    // describe a thrown value that is no error by its text
    if (!(error instanceof Error)) {
        return { code: "BUILD_FAILED", message: String(error) };
    }

    return {
        code: error instanceof BuildError ? error.code : "BUILD_FAILED",
        message: error.message,
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
function writeDiagnostics(...messages: (string | ArrayBufferView | ArrayBuffer)[]): number {
    // write at once, as the console's write answers the bytes written
    const bytes = Buffer.concat(messages.map((message) => toBuffer(message)));
    writeSync(2, bytes);

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
