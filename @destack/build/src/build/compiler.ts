import { readFile, writeFile } from "node:fs/promises";
import { resolve, join } from "node:path";
import { fileURLToPath } from "node:url";
import { type DependencyResolution, type Package } from "@destack/package";
import type {
    BuildExtension,
    ExpandedOutput,
    OutputKind,
    OutputRequest,
} from "@destack/package/build";
import { type BuildDescription, type DeclarationDescription } from "@destack/package/inspect";
import { describeFile } from "@destack/package/file";
import { type PackageOutput, type PackageManifest } from "@destack/package/manifest";
import { type SourceMapReference } from "@destack/package/source";
import { PackageLocator } from "@destack/package/transform";
import { checkPackage, formatPackage } from "@destack/check";
import { type TypeScriptInspection, TypeScriptCompiler } from "../typescript/index.ts";
import { inspectModules, type InspectOptions } from "../inspect/inspection.ts";
import { Upgrade, type History } from "@destack/resource";
import {
    evaluateDeclarations,
    kindKey,
    selectDeclarations,
    type Evaluation,
} from "../declaration/index.ts";
import { type PackageSource, openSource, readPackageDescription } from "../source/index.ts";
import {
    OutputCompilation,
    type CompiledFiles,
    type ModuleOptions,
} from "../compile/compilation.ts";
import { loadExtensions } from "../compile/extension.ts";
import { OutputPass } from "../compile/pass.ts";
import { checkRuntime, RuntimeCompiler } from "../compile/runtime.ts";
import { Template } from "../template/index.ts";
import { BuildError } from "../error/index.ts";
import { PackageBuild, type BuildOptions } from "./build.ts";
import { BuildFiles } from "./files.ts";
import { comparePath, stringifyInspection } from "./serialization.ts";
import {
    encodeDescription,
    MANIFEST_INVENTORIES,
    serializeDescriptions,
    type ManifestDescription,
} from "./manifest.ts";

/** An output a build compiles: a module output, or one output of a kind's pass. */
interface PlannedOutput {
    /** The output's module settings. */
    readonly options: ModuleOptions;
    /** Package-relative modules inspected beside the entries. */
    readonly files: readonly string[];
    /** The kind's request compiling the output in a pass of its own. */
    readonly pass?: {
        /** The requested output's name, shared by the outputs the kind expands it into. */
        readonly name: string;
        /** The output kind compiling the pass. */
        readonly kind: OutputKind;
        /** The request with the kind's settings. */
        readonly request: OutputRequest;
    };
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
    /** The Destack packages the build tool's own dependencies resolve to. */
    readonly #packages = new PackageLocator();
    /** Configurations indexed by output settings. */
    readonly #sources = new Map<string, { declaration: string; source: PackageSource }>();

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
        );

        // evaluate the collected domain declarations
        const evaluation = await evaluateDeclarations(declarations, project);

        return inspectModules(project.declaration.package, modules, tests, evaluation.declarations);
    }

    /** Inspect a source package and compile its requested outputs. */
    async build(options: BuildOptions, destination: string): Promise<PackageBuild> {
        // plan the outputs with the extensions of the package's dependencies
        const declaration = await readPackageDescription(options.directory);
        const extensions = await loadExtensions(options.directory, declaration);
        const planned = expand(options.outputs, extensions);

        // inspect, check and evaluate every output's sources, then retain them before compiling any
        const inspected = await this.#inspect(options, planned);
        const files = new BuildFiles(destination);
        await retain(inspected, files);

        // compile the outputs, then describe the declarations each selects
        const compiled = await this.#compile(options, planned, inspected, extensions, files);
        const description = await this.#describe(planned, inspected, compiled);

        // plan the upgrade from what the package has published
        const source = inspected.values().next().value!.project.declaration.package;
        const upgrade =
            options.history === undefined
                ? undefined
                : planUpgrade(options.history, source, description.manifest, inspected);

        // write the manifest
        return await writeManifest(source, compiled, description, upgrade, files, this.#packages);
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
        let configuration: string | undefined;
        try {
            configuration = await readFile(
                resolve(this.directory, options.configuration ?? "tsconfig.json"),
                "utf8",
            );
        } catch (error) {
            if (!(error instanceof Error && "code" in error && error.code === "ENOENT")) {
                throw error;
            }
        }

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
                .map((result) => result.reason);
            if (failures.length) {
                throw new AggregateError(failures, "compiler shutdown failed");
            }
        } finally {
            await Promise.all(
                [...this.#sources.values()].map(({ source }) => source[Symbol.asyncDispose]()),
            );
        }
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
            const project = await this.open({
                directory: options.directory,
                configuration: options.configuration,
                runtime: output.options.runtime,
                entries: output.options.entries,
                files: output.files,
            });

            // refuse a package that changes identity while the build runs
            const identity = project.declaration.package;
            if (
                source &&
                (source.id !== identity.id ||
                    source.name !== identity.name ||
                    source.version !== identity.version)
            ) {
                throw new BuildError("BUILD_FAILED", "package changed during build");
            }
            source = identity;

            // reuse identical inspections within this build
            let inspection = inspections.get(project.configuration);
            if (!inspection) {
                inspection = await this.typescript.inspect(project.configuration);
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

    /** Compile module outputs one by one, and each kind's outputs in one pass. */
    async #compile(
        options: BuildOptions,
        planned: ReadonlyMap<string, PlannedOutput>,
        inspected: ReadonlyMap<string, InspectedOutput>,
        extensions: readonly BuildExtension[],
        files: BuildFiles,
    ): Promise<{
        outputs: Map<string, Pick<CompiledFiles, "output" | "inspection">>;
        sourceMaps: SourceMapReference[];
    }> {
        // collect each output's description and every source map
        const outputs = new Map<string, Pick<CompiledFiles, "output" | "inspection">>();
        const sourceMaps: SourceMapReference[] = [];
        for (const [name, output] of planned) {
            // compile a module output with the extensions' plugins
            if (output.pass === undefined) {
                const compilation = createCompilation(name, output, inspected, extensions);
                const compiled = await compilation.compile(
                    options.dependencies,
                    files.retained,
                    files.directory,
                );
                await keep(compiled.files, compiled.paths, files);
                sourceMaps.push(...compiled.sourceMaps);
                outputs.set(name, { output: compiled.output, inspection: compiled.inspection });
            }
            // compile a kind's outputs together at its first output
            else if (!outputs.has(name)) {
                const pass = await this.#pass(
                    output.pass,
                    options,
                    planned,
                    inspected,
                    extensions,
                    files,
                );
                sourceMaps.push(...pass.sourceMaps);
                for (const [member, described] of pass.outputs) {
                    outputs.set(member, described);
                }
            }
        }

        return { outputs, sourceMaps };
    }

    /** Compile the outputs of one kind's request in its own pass. */
    async #pass(
        request: NonNullable<PlannedOutput["pass"]>,
        options: BuildOptions,
        planned: ReadonlyMap<string, PlannedOutput>,
        inspected: ReadonlyMap<string, InspectedOutput>,
        extensions: readonly BuildExtension[],
        files: BuildFiles,
    ): Promise<{
        outputs: Map<string, Pick<CompiledFiles, "output" | "inspection">>;
        sourceMaps: SourceMapReference[];
    }> {
        // lend the kind the build's parts for the outputs its request expanded into
        const members = [...planned.keys()].filter(
            (member) => planned.get(member)!.pass?.name === request.name,
        );
        const compilations = Object.fromEntries(
            members.map((member) => [
                member,
                createCompilation(member, planned.get(member)!, inspected, extensions),
            ]),
        );
        const pass = new OutputPass(
            inspected.get(members[0]!)!.project,
            options.dependencies,
            files.retained,
            files.directory,
            compilations,
            this.runtimes,
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
        const outputs = new Map<string, Pick<CompiledFiles, "output" | "inspection">>();
        for (const member of members) {
            const output = described[member];
            if (output === undefined) {
                throw new BuildError("BUILD_FAILED", `the pass described no output ${member}`);
            }
            outputs.set(member, { output, inspection: pass.inspection(member) });
        }

        return { outputs, sourceMaps: pass.sourceMaps };
    }

    /** Check each module output against its runtime, and collect the declarations each selects. */
    async #describe(
        planned: ReadonlyMap<string, PlannedOutput>,
        inspected: ReadonlyMap<string, InspectedOutput>,
        compiled: { outputs: ReadonlyMap<string, Pick<CompiledFiles, "output" | "inspection">> },
    ): Promise<{ manifest: ManifestDescription; resolved: Record<string, DependencyResolution> }> {
        // share equal descriptions while retaining target-specific variants
        const modules = new SharedValues<ManifestDescription["modules"][number]>();
        const declarations = new SharedValues<DeclarationDescription>();
        const tests = new SharedValues<ManifestDescription["tests"][number]>();
        const selections: ManifestDescription["selections"] = new Map();
        const resolved: Record<string, DependencyResolution> = {};
        for (const [name, { output, inspection: description }] of compiled.outputs) {
            // require every dependency declaration at its exact resolved version
            const { project, inspection, evaluation } = inspected.get(name)!;
            const source = project.declaration.package;
            const selected = selectDeclarations(evaluation.declarations, source, description);
            for (const declaration of selected) {
                requirePresent(declaration, source, description, output);
            }

            // check module outputs against their runtime, kinds checking their own
            if (planned.get(name)!.pass === undefined) {
                await checkRuntime(description, inspection.modules, project.runtime, this.runtimes);
            }

            // retain exact dependency resolutions once, joining the files outputs read of a source
            for (const [key, dependency] of Object.entries(description.packages)) {
                resolved[key] = joinResolutions(key, resolved[key], dependency);
            }
            selections.set(name, {
                modules: inspection.modules.map((value) => modules.retain(value)),
                declarations: selected.map((value) => declarations.retain(value)),
                tests: inspection.tests.map((value) => tests.retain(value)),
            });
        }

        return {
            manifest: {
                modules: modules.values,
                declarations: declarations.values,
                tests: tests.values,
                modulePackage: await findPackage("@destack/package", this.#packages),
                testPackage: await findPackage("@destack/test", this.#packages),
                selections,
            },
            resolved,
        };
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
    // refuse an invalid or taken output name
    const planned = new Map<string, PlannedOutput>();
    const add = (name: string, output: PlannedOutput) => {
        if (!/^[a-z][a-z0-9-]*$/.test(name)) {
            throw new BuildError("BUILD_FAILED", `invalid output name: ${name}`);
        }
        if (planned.has(name)) {
            throw new BuildError("BUILD_FAILED", `duplicate output name: ${name}`);
        }
        planned.set(name, output);
    };
    for (const [name, request] of Object.entries(requests)) {
        // plan a module output
        if (request.kind === "module") {
            add(name, { options: request as ModuleOptions, files: [] });
        }
        // plan the outputs a kind expands its request into
        else {
            const kind = kindOf(request.kind, extensions);
            const requested = request as OutputRequest;
            let expanded: Readonly<Record<string, ExpandedOutput>>;
            try {
                expanded = kind.expand(name, requested);
            } catch (cause) {
                throw BuildError.from(cause);
            }
            for (const [member, output] of Object.entries(expanded)) {
                const { files, ...settings } = output;
                add(member, {
                    options: { kind: "module", ...settings },
                    files,
                    pass: { name, kind, request: requested },
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
    const { project, inspection, evaluation } = inspected.get(name)!;

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

/** Reject source diagnostics and uncanonical formatting before compiling publication outputs. */
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
        signal: options.signal,
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

/** Retain every inspected source, template asset and authored manifest before compiling. */
async function retain(
    inspected: ReadonlyMap<string, InspectedOutput>,
    files: BuildFiles,
): Promise<void> {
    for (const { project, inspection } of inspected.values()) {
        // retain inspected sources
        for (const [path, bytes] of inspection.sources) {
            files.retain(path, bytes);
        }

        // retain template assets
        if (project.declaration.definition.template) {
            const template = await Template.read(project.directory);
            for (const [path, bytes] of template.files) {
                files.retain(path, new Uint8Array(bytes));
            }
        }

        // retain the authored package declarations
        for (const path of ["package.json", "destack.json"]) {
            files.retain(path, new Uint8Array(await readFile(resolve(project.directory, path))));
        }
    }
}

/** Write a compilation's generated files, and record the files its compiler wrote. */
async function keep(
    generated: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
    paths: readonly string[],
    files: BuildFiles,
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
    // accept the package's own declarations
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

/** Serialize the descriptions and inventories, then write the manifest referring to them. */
async function writeManifest(
    source: Package,
    compiled: {
        outputs: ReadonlyMap<string, Pick<CompiledFiles, "output" | "inspection">>;
        sourceMaps: SourceMapReference[];
    },
    description: { manifest: ManifestDescription; resolved: Record<string, DependencyResolution> },
    upgrade: Upgrade | undefined,
    files: BuildFiles,
    locator: PackageLocator,
): Promise<PackageBuild> {
    // serialize shared descriptions by domain and module
    const outputs = Object.fromEntries(
        [...compiled.outputs].map(([name, { output }]) => [name, output]),
    );
    const manifest = await serializeDescriptions(description.manifest, outputs);

    // write the retained sources and describe the distributed files with their modules
    await files.flush();
    const described = files.list().map((file) => {
        const inspected = manifest.modules.get(file.path);

        return inspected ? { ...file, descriptions: inspected } : file;
    });

    // require every inspected path to identify a distributed file
    for (const path of manifest.modules.keys()) {
        if (!files.has(path)) {
            throw new BuildError("BUILD_FAILED", `inspected file is absent from build: ${path}`);
        }
    }

    // publish inventories separately from source and executable files
    const inventories = {
        dependencies: description.resolved,
        files: described,
        sourceMaps: compiled.sourceMaps,
    };
    const references = {} as Pick<PackageManifest, "dependencies" | "files" | "sourceMaps">;
    for (const name of MANIFEST_INVENTORIES) {
        const path = `manifest/${name}.json`;
        const bytes = encodeDescription(inventories[name]);
        references[name] = await describeFile(path, "application/json", bytes);
        await files.write(path, bytes);
    }
    for (const [path, bytes] of manifest.files) {
        await files.write(path, bytes);
    }

    // keep the upgrade from the published release in its own file
    let upgraded: PackageManifest["upgrade"];
    if (upgrade !== undefined) {
        const path = "manifest/upgrade.json";
        const bytes = encodeDescription(upgrade);
        upgraded = {
            package: await findPackage("@destack/resource", locator),
            file: await describeFile(path, "application/json", bytes),
        };
        await files.write(path, bytes);
    }

    // assemble the manifest
    const result: PackageManifest = {
        formatVersion: 1,
        package: source,
        language: "typescript",
        dependencies: references.dependencies,
        descriptions: manifest.descriptions,
        tests: manifest.tests,
        upgrade: upgraded,
        outputs,
        files: references.files,
        sourceMaps: references.sourceMaps,
    };
    await writeFile(join(files.directory, "manifest.json"), JSON.stringify(result), { flag: "wx" });

    return new PackageBuild(result, files.directory, "retained");
}

/** Plan the upgrade from what the package has published. */
function planUpgrade(
    history: History,
    source: Package,
    manifest: ManifestDescription,
    inspected: ReadonlyMap<string, InspectedOutput>,
): Upgrade {
    // compare the package's own declarations
    const compare = new Map(
        [...inspected.values()].flatMap((output) => [...output.evaluation.compare]),
    );
    const declarations = manifest.declarations.filter(
        (declaration) => declaration.symbol.package.id === source.id,
    );

    return Upgrade.plan(history, declarations, (declaration) => compare.get(kindKey(declaration)));
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
    const ordered = [...files.values()].sort(comparePath);

    return { ...next, files: ordered };
}

/** Read the identity of a Destack package resolved from the build tool's dependencies. */
async function findPackage(specifier: string, locator: PackageLocator): Promise<Package> {
    const owner = await locator.find(fileURLToPath(import.meta.resolve(specifier)));
    if (!owner) {
        throw new BuildError("BUILD_FAILED", `missing Destack package: ${specifier}`);
    }

    return owner.metadata.package;
}
