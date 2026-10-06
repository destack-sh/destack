import { readFile, realpath } from "node:fs/promises";
import type { TestDeclaration } from "@destack/test/inspect";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
    type DependencyResolution,
    graph,
    type Package,
    type PackageId,
    type DeclarationDescription,
} from "@destack/package";
import type { ModuleDescription } from "@destack/package/code";
import { aligned, type Commit, Digest, found, present, schema } from "@destack/schema";
import {
    type BuildExtension,
    type ExpandedOutput,
    type OutputKind,
    type OutputRequest,
    type BuildDescription,
} from "@destack/package/build";
import { type PackageOutput } from "@destack/package/manifest";
import { type SourceMapReference } from "@destack/package/source";
import { PackageLocator } from "@destack/package/transform";
import { checkPackage, formatPackage, Toolchain } from "@destack/check";
import {
    type ExampleDeclaration,
    type ProgramImports,
    type ScenarioDeclaration,
    type TypeScriptInspection,
    TypeScriptCompiler,
} from "../typescript/index.ts";
import { inspectModules, type InspectOptions } from "../inspect/inspection.ts";
import { Upgrade, type History } from "@destack/resource";
import {
    evaluateDeclarations,
    kindKey,
    selectDeclarations,
    type Evaluation,
} from "../declaration/index.ts";
import {
    compiledPackage,
    type PackageSource,
    openSource,
    readPackageDescription,
} from "../source/index.ts";
import { OutputCompilation, type CompiledFiles } from "../compile/compilation.ts";
import { type LoadedExtension, loadExtensions } from "../compile/extension.ts";
import {
    type BuildKeys,
    type CachedOutput,
    deriveModuleKeys,
    Identities,
    identifyToolchain,
    importClosure,
} from "../cache/index.ts";
import { OutputPass } from "../compile/pass.ts";
import { checkRuntime, RuntimeCompiler } from "../compile/runtime.ts";
import { Template } from "../template/index.ts";
import { BuildError, isMissing } from "../error/index.ts";
import { PackageBuild, type BuildOptions, type ModuleOptions } from "./build.ts";
import { BuildDirectory } from "./directory.ts";
import { readCatalogs } from "./catalog.ts";
import { comparePath, stringifyInspection } from "./serialization.ts";
import { describeGraph } from "../graph/module.ts";

/** This package's directory, whose dependencies hold the tools when running from a workspace. */
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

/** Compiler state and configurations retained for one source package. */
export class BuildCompiler implements AsyncDisposable {
    /** The native TypeScript compiler. */
    readonly typescript: TypeScriptCompiler;
    /** Runtime type environments reused across output checks. */
    readonly runtimes = new RuntimeCompiler();
    /** The source package directory. */
    readonly directory: string;
    /** The Destack packages the build tool's dependencies resolve to. */
    readonly #packages = new PackageLocator();
    /** Configurations indexed by output settings. */
    readonly #sources = new Map<string, { declaration: string; source: PackageSource }>();
    /** The identity of the toolchain this process runs, read once. */
    #toolchain: Promise<Digest> | undefined;
    /** The identity of each loaded extension's package, read once, since the process keeps the code it first imported. */
    readonly #extensions = new Map<string, Promise<Digest>>();

    /** Start the compiler for one source checkout. */
    constructor(directory: string) {
        this.directory = resolve(directory);
        this.typescript = new TypeScriptCompiler(this.directory);
    }

    /** Inspect source using retained compiler state and source configurations. */
    async inspect(options: InspectOptions) {
        // inspect source with the selected compiler configuration
        const project = await this.open(options);
        const { modules, tests, declarations } = await this.typescript.inspect(
            project.configuration,
            project.entries,
            compiledPackage(project),
        );

        // evaluate the collected domain declarations
        const evaluation = await evaluateDeclarations(declarations, project);

        return inspectModules(project.declaration.package, modules, tests, evaluation.declarations);
    }

    /** Identify a build's inputs beside its modules: the toolchain, the extensions, the package's manifests and its catalogs. */
    async #identifyInputs(
        directory: string,
        packageId: PackageId,
        loaded: Awaited<ReturnType<typeof loadExtensions>>,
        identities: Identities,
    ) {
        this.#toolchain ??= identifyToolchain(new Identities());
        const files = (paths: Iterable<string>) =>
            Promise.all([...paths].map((path) => identities.file(join(directory, path))));

        return {
            toolchain: await this.#toolchain,
            extensions: await Promise.all(loaded.map((entry) => this.#identify(entry))),
            manifests: await files(["package.json", "destack.json"]),
            catalogs: await files((await readCatalogs(directory, packageId)).keys()),
        };
    }

    /** Derive the cache keys of a build without checking, evaluating or compiling it. */
    async plan(options: BuildOptions): Promise<BuildKeys> {
        // plan the outputs with the extensions of the package's dependency closure
        const declaration = await readPackageDescription(options.directory);
        const loaded = await loadExtensions(options.directory, declaration);
        const planned = expand(
            options.outputs,
            loaded.map((entry) => entry.extension),
        );

        // identify the toolchain, the extensions, the package's manifests and its catalogs
        const identities = new Identities();
        const { toolchain, extensions, manifests, catalogs } = await this.#identifyInputs(
            options.directory,
            declaration.package.id,
            loaded,
            identities,
        );

        // key each requested output by its request, its toolchain and the modules it imports
        const { modules, outputs } = await this.#keyModules(
            options,
            planned,
            toolchain,
            identities,
        );
        const keys: Record<string, Digest> = {};
        for (const [request, imported] of outputs) {
            keys[request] = await Digest.json({
                toolchain,
                extensions,
                manifests,
                version: options.version ?? null,
                request: present(options.outputs[request], `the requested output ${request}`),
                configuration: options.configuration ?? null,
                modules: [...imported].toSorted(),
            });
        }

        // digest the template a template package publishes
        const template = declaration.definition.template
            ? await digestTemplate(options.directory)
            : null;

        // key the build by every input that changes it
        const build = await Digest.json({
            toolchain,
            extensions,
            manifests,
            version: options.version ?? null,
            catalogs,
            outputs: keys,
            modules: modules.toSorted(),
            dependencies: options.dependencies,
            history: options.history ?? null,
            commit: options.commit ?? null,
            template,
        });

        return { build, outputs: keys };
    }

    /** Inspect a source package and compile its requested outputs, reusing the cached ones. */
    async build(
        options: BuildOptions,
        reuse: Readonly<Record<string, CachedOutput>>,
        destination: string,
    ): Promise<{ build: PackageBuild; outputs: Record<string, CachedOutput> }> {
        // plan the outputs with the extensions of the package's dependency closure
        const declaration = await readPackageDescription(options.directory);
        const extensions = (await loadExtensions(options.directory, declaration)).map(
            (entry) => entry.extension,
        );
        const planned = expand(options.outputs, extensions);

        // inspect, check, evaluate and retain every output's sources before compiling any
        const inspected = await this.#inspect(options, planned);
        const files = new BuildDirectory(destination);
        await retain(inspected, files);

        // compile the outputs the cache lacks
        const compiled = await this.#compile(options, planned, inspected, extensions, files, reuse);

        // describe the declarations each output selects
        const description = await this.#describe(planned, inspected, compiled);

        // plan the upgrade from what the package has published
        const source = sourcePackage(inspected);
        const upgrade =
            options.history === undefined
                ? undefined
                : planUpgrade(options.history, source, description.graph, inspected);

        // write the manifest
        const build = await writeManifest(
            source,
            options.commit,
            compiled,
            description,
            upgrade,
            files,
            this.#packages,
        );

        return { build, outputs: compiled.cached };
    }

    /** Reuse a configuration until its package declarations change. */
    async open(options: Parameters<typeof openSource>[0]): Promise<PackageSource> {
        // read the manifests on every build so changed exports update the compiler roots
        const manifests = await Promise.all(
            ["package.json", "destack.json"].map((path) =>
                readFile(join(this.directory, path), "utf8"),
            ),
        );

        // include authored compiler settings in configuration invalidation
        const configuration = await readOptional(
            resolve(this.directory, options.configuration ?? "tsconfig.json"),
        );

        // reuse the configuration while its inputs remain unchanged
        const declaration = JSON.stringify([manifests, configuration]);
        const key = JSON.stringify(options);
        const previous = this.#sources.get(key);
        if (previous?.declaration === declaration) {
            return previous.source;
        }

        // replace changed configurations and retain the new source until shutdown
        const source = await openSource(options);
        await previous?.source[Symbol.asyncDispose]();
        this.#sources.set(key, { declaration, source });

        return source;
    }

    /** Close the native compiler before releasing its configurations. */
    async [Symbol.asyncDispose](): Promise<void> {
        try {
            // await both compiler shutdowns before releasing source configurations
            const results = await Promise.allSettled([
                this.typescript[Symbol.asyncDispose](),
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

    /** Open the package with an output's entries and its kind's extra modules as roots. */
    #project(options: BuildOptions, output: PlannedOutput): Promise<PackageSource> {
        return this.open({
            directory: options.directory,
            ...(options.configuration === undefined
                ? {}
                : { configuration: options.configuration }),
            runtime: output.options.runtime,
            ...(output.options.entries === undefined ? {} : { entries: output.options.entries }),
            files: output.files,
            ...(options.version === undefined ? {} : { version: options.version }),
        });
    }

    /** Key the modules of each distinct program once, and collect the keys of the modules each requested output imports. */
    async #keyModules(
        options: BuildOptions,
        planned: ReadonlyMap<string, PlannedOutput>,
        toolchain: Digest,
        identities: Identities,
    ): Promise<{ modules: Digest[]; outputs: Map<string, Set<Digest>> }> {
        // collect the keys of every module and of each output's modules
        const programs = new Map<string, Promise<ProgramKeys>>();
        const modules = new Map<string, Digest>();
        const outputs = new Map<string, Set<Digest>>();
        for (const [name, output] of planned) {
            // key each program's modules once
            const project = await this.#project(options, output);
            let program = programs.get(project.configuration);
            if (program === undefined) {
                program = this.#keys(project, toolchain, identities);
                programs.set(project.configuration, program);
            }
            const { imports, keys } = await program;
            for (const [file, key] of keys) {
                modules.set(`${project.configuration}\0${file}`, key);
            }

            // collect the keys of the modules each requested output imports
            const request = output.pass?.name ?? name;
            const imported = outputs.get(request) ?? new Set();
            const entries = await Promise.all(project.entries.map((entry) => realpath(entry)));
            for (const file of importClosure(imports, entries)) {
                imported.add(found(keys, file));
            }
            outputs.set(request, imported);
        }

        return { modules: [...modules.values()], outputs };
    }

    /** Read a program's imports and key its modules. */
    async #keys(
        project: PackageSource,
        toolchain: Digest,
        identities: Identities,
    ): Promise<ProgramKeys> {
        const imports = await this.typescript.imports(project.configuration);
        const keys = await deriveModuleKeys(imports, project.directory, toolchain, identities);

        return { imports, keys };
    }

    /** Identify an extension's package once, as this process keeps the extension it first imported. */
    #identify(loaded: LoadedExtension): Promise<Digest> {
        let identity = this.#extensions.get(loaded.directory);
        if (identity === undefined) {
            identity = new Identities().package(loaded.directory);
            this.#extensions.set(loaded.directory, identity);
        }

        return identity;
    }

    /** Open and inspect each output, check every source, then evaluate each distinct inspection once. */
    async #inspect(
        options: BuildOptions,
        planned: ReadonlyMap<string, PlannedOutput>,
    ): Promise<Map<string, InspectedOutput>> {
        // share equal inspections across outputs
        const inspections = new Map<string, TypeScriptInspection>();
        const opened = new Map<string, Omit<InspectedOutput, "evaluation">>();
        let source: Package | undefined;
        for (const [name, output] of planned) {
            // open the package with the output's entries and the kind's extra modules as roots
            const project = await this.#project(options, output);

            // refuse a package that changes identity while the build runs
            const identity = project.declaration.package;
            if (source !== undefined && !isSamePackage(source, identity)) {
                throw new BuildError("BUILD_FAILED", "package changed during build");
            }
            source = identity;

            // reuse identical inspections within this build
            let inspection = inspections.get(project.configuration);
            if (!inspection) {
                inspection = await this.typescript.inspect(
                    project.configuration,
                    project.entries,
                    compiledPackage(project),
                );
                inspections.set(project.configuration, inspection);
            }
            opened.set(name, { project, inspection });
        }

        // reject source diagnostics and formatting before running any package code
        await check(options, opened);

        // evaluate each distinct inspection's declarations once
        const evaluated = new Map<TypeScriptInspection, Evaluation>();
        const inspected = new Map<string, InspectedOutput>();
        for (const [name, { project, inspection }] of opened) {
            let evaluation = evaluated.get(inspection);
            if (!evaluation) {
                evaluation = await evaluateDeclarations(inspection.declarations, project);
                evaluated.set(inspection, evaluation);
            }
            inspected.set(name, { project, inspection, evaluation });
        }

        return inspected;
    }

    /** Compile each requested output the cache lacks, a kind's outputs in one pass, and reuse the others. */
    async #compile(
        options: BuildOptions,
        planned: ReadonlyMap<string, PlannedOutput>,
        inspected: ReadonlyMap<string, InspectedOutput>,
        extensions: readonly BuildExtension[],
        files: BuildDirectory,
        reuse: Readonly<Record<string, CachedOutput>>,
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

            // keep a request compiled or reused before
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
                        ? await this.#module(name, output, options, inspected, extensions, files)
                        : await this.#pass(
                              output.pass,
                              options,
                              planned,
                              inspected,
                              extensions,
                              files,
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

    /** Compile a module output with the extensions' plugins and keep its files. */
    async #module(
        name: string,
        output: PlannedOutput,
        options: BuildOptions,
        inspected: ReadonlyMap<string, InspectedOutput>,
        extensions: readonly BuildExtension[],
        files: BuildDirectory,
    ): Promise<CachedOutput> {
        // compile the output and keep its files
        const compilation = createCompilation(name, output, inspected, extensions);
        const compiled = await compilation.compile(
            options.dependencies,
            files.retained,
            files.directory,
        );
        await keep(compiled.files, compiled.paths, files);

        return {
            outputs: { [name]: { output: compiled.output, description: compiled.inspection } },
            sourceMaps: compiled.sourceMaps,
            files: [...compiled.files.keys(), ...compiled.paths].map((path) => files.file(path)),
        };
    }

    /** Compile the outputs of one kind's request in a separate pass and keep their files. */
    async #pass(
        request: NonNullable<PlannedOutput["pass"]>,
        options: BuildOptions,
        planned: ReadonlyMap<string, PlannedOutput>,
        inspected: ReadonlyMap<string, InspectedOutput>,
        extensions: readonly BuildExtension[],
        files: BuildDirectory,
    ): Promise<CachedOutput> {
        // lend the kind the build's inputs for the outputs its request expanded into
        const plans = [...planned].filter(([, plan]) => plan.pass?.name === request.name);
        const members = plans.map(([member]) => member);
        const compilations = Object.fromEntries(
            plans.map(([member, plan]) => [
                member,
                createCompilation(member, plan, inspected, extensions),
            ]),
        );
        const pass = new OutputPass(
            found(inspected, aligned(members, 0)).project,
            options.dependencies,
            files.retained,
            files.directory,
            compilations,
            this.runtimes,
            extensions,
        );

        // compile the outputs and keep their files
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
        await keep(pass.files, pass.paths, files);

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

            // keep each module's first description and every evaluated declaration for the graph
            retainGraphInputs(entry, modules, evaluated);

            // keep each test, example and scenario once across outputs
            for (const test of entry.inspection.tests) {
                tests.retain(test);
            }
            for (const example of entry.inspection.examples) {
                examples.retain(example);
            }
            for (const scenario of entry.inspection.scenarios) {
                scenarios.retain(scenario);
            }
        }

        // describe the package's graph from its modules, every evaluated declaration, its tests, examples and scenarios
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

    /** Require an output's dependency declarations at their resolved versions and check its runtime. */
    async #checkOutput(
        inspected: InspectedOutput,
        planned: PlannedOutput,
        output: PackageOutput,
        description: BuildDescription,
    ): Promise<void> {
        // require every dependency declaration at its exact resolved version
        const { project, inspection, evaluation } = inspected;
        const source = project.declaration.package;
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
}

/** Keep each module's first description and every evaluated declaration of an output. */
function retainGraphInputs(
    inspected: InspectedOutput,
    modules: Map<string, ModuleDescription>,
    evaluated: Set<DeclarationDescription>,
): void {
    // NOTE #Incomplete: a module checked differently per runtime keeps the first output's graph
    for (const module of inspected.inspection.modules) {
        if (!modules.has(module.path)) {
            modules.set(module.path, module);
        }
    }
    for (const declaration of inspected.evaluation.declarations) {
        evaluated.add(declaration);
    }
}

/** Read the package every inspected output opens. */
function sourcePackage(inspected: ReadonlyMap<string, InspectedOutput>): Package {
    return present(inspected.values().next().value, "an inspected output").project.declaration
        .package;
}

/** Read a source's package.json as its build releases it: as authored, or at the release's version. */
async function releasedManifest(project: PackageSource): Promise<Uint8Array<ArrayBuffer>> {
    // keep the authored bytes of the version they name
    const authored = await readFile(resolve(project.directory, "package.json"), "utf8");
    const manifest = schema.record(schema.string(), schema.json()).parse(JSON.parse(authored));
    const { version } = project.declaration.package;
    if (manifest["version"] === version) {
        return new TextEncoder().encode(authored);
    }

    return new TextEncoder().encode(`${JSON.stringify({ ...manifest, version }, null, 4)}\n`);
}

/** Report whether two identities name the same package release. */
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

/** Values shared by index across outputs, each distinct value kept once. */
class SharedValues<Value> {
    /** The distinct values, in order of first appearance. */
    readonly values: Value[] = [];
    /** The index of each value, by its serialization. */
    readonly #indices = new Map<string, number>();

    /** Keep a value once, returning its index. */
    retain(value: Value): number {
        // reuse the index of an equal value
        const key = stringifyInspection(value);
        const existing = this.#indices.get(key);
        if (existing !== undefined) {
            return existing;
        }

        // keep a new value
        const index = this.values.length;
        this.values.push(value);
        this.#indices.set(key, index);

        return index;
    }
}

/** Expand the requested outputs into module outputs and the outputs of each kind's pass. */
function expand(
    requests: BuildOptions["outputs"],
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
    );
}

/** Reject source diagnostics and uncanonical formatting before compiling shipped outputs. */
async function check(
    options: BuildOptions,
    inspected: ReadonlyMap<string, Pick<InspectedOutput, "inspection">>,
): Promise<void> {
    // reject diagnostics in every inspected source
    const request = {
        directory: options.directory,
        files: [
            ...new Set(
                [...inspected.values()].flatMap(({ inspection }) => [...inspection.sources.keys()]),
            ),
        ],
        ...(options.signal === undefined ? {} : { signal: options.signal }),
    };
    const checked = await checkPackage(request);
    if (checked.diagnostics.length) {
        throw new BuildError("BUILD_FAILED", JSON.stringify(checked));
    }

    // require canonical formatting without changing source
    const formatted = await formatPackage(request, false);
    if (formatted.code !== 0) {
        throw new BuildError("BUILD_FAILED", formatted.stdout + formatted.stderr);
    }
}

/** Retain every inspected source, template asset, authored manifest and catalog before compiling. */
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
        if (project.declaration.definition.template) {
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
        const owner = project.declaration.package.id;
        for (const [path, bytes] of await readCatalogs(project.directory, owner)) {
            files.retain(path, bytes);
        }
    }
}

/** Write a compilation's generated files, and record the files its compiler wrote. */
async function keep(
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
): Promise<PackageBuild> {
    // write the build in its format, the upgrade qualified by the package defining it
    const manifest = await files.finish({
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

    return new PackageBuild(manifest, files.directory, "retained");
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

/** Join two outputs' resolutions of one dependency, uniting the files they read of a source package. */
function joinResolutions(
    key: string,
    previous: DependencyResolution | undefined,
    next: DependencyResolution,
): DependencyResolution {
    // keep the first resolution and accept an equal one
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

/** Digest a template package's files by path. */
async function digestTemplate(directory: string): Promise<[string, Digest][]> {
    const template = await Template.read(directory);

    return await Promise.all(
        [...template.files].map(async ([path, bytes]): Promise<[string, Digest]> => [
            path,
            await Digest.of(new Uint8Array(bytes)),
        ]),
    );
}

/** Read the identity of a Destack package among the build tool's dependencies. */
async function findPackage(name: string, locator: PackageLocator): Promise<Package> {
    const owner = await locator.find(join(Toolchain.locate([name], PACKAGE), "package.json"));
    if (!owner) {
        throw new BuildError("BUILD_FAILED", `missing Destack package: ${name}`);
    }

    return owner.metadata.package;
}
