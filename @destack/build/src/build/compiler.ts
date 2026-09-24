import { readFile, mkdir, writeFile } from "node:fs/promises";
import { createReadStream } from "node:fs";
import { createHash } from "node:crypto";
import { resolve, join, dirname } from "node:path";
import { type Package } from "@destack/package";
import { describeFile, type PackageFile } from "@destack/package/file";
import { type PackageOutput, type PackageManifest } from "@destack/package/manifest";
import type { DependencyResolution } from "@destack/package";
import { type SourceMapReference } from "@destack/package/source";
import { type TypeScriptInspection, TypeScriptCompiler } from "../typescript/index.ts";
import { inspectModules, type InspectOptions } from "../inspect/inspection.ts";
import { stringifyInspection } from "./serialization.ts";
import { evaluateDeclarations, selectDeclarations } from "../declaration/index.ts";
import { type BuildDescription, type DeclarationDescription } from "@destack/package/inspect";
import { type PackageSource, openPackage } from "../source/index.ts";
import { type CompileOptions, compilePackage } from "../compile/index.ts";
import { BuildError } from "../error/index.ts";
import { PackageBuild, type BuildOptions } from "./build.ts";
import { type ApplicationOptions, compileApplication } from "../compile/application.ts";
import { describeWorkloads } from "./workload.ts";
import { Template } from "../template/index.ts";
import { resolveView } from "../source/view.ts";
import { checkRuntime, RuntimeCompiler } from "../compile/runtime.ts";
import { checkPackage, formatPackage } from "@destack/check";
import { serializeDescriptions, encodeDescription, type ManifestDescription } from "./manifest.ts";
import { ModulePackages } from "@destack/package/transform";
import { fileURLToPath } from "node:url";

/** Compiler state and configurations retained for one source package. */
export class BuildCompiler implements AsyncDisposable {
    /** The native TypeScript compiler. */
    readonly typescript: TypeScriptCompiler;
    /** Runtime type environments reused across output checks. */
    readonly runtimes = new RuntimeCompiler();
    /** The source package directory. */
    readonly directory: string;
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
        const descriptions = await evaluateDeclarations(declarations, project);

        return inspectModules(project.declaration.package, modules, tests, descriptions);
    }

    /** Inspect a source package and compile its requested outputs. */
    async build(options: BuildOptions, destination: string): Promise<PackageBuild> {
        // collect distributed files and named outputs
        const dependencies = options.dependencies;
        const files = new Map<string, Uint8Array<ArrayBuffer>>();
        const records = new Map<string, PackageFile>();
        const outputs: Record<string, PackageOutput> = {};
        const sourceMaps: SourceMapReference[] = [];
        let source: Package | undefined;

        // retain web compilations shared by browser and server outputs
        const applications = new Map<string, Awaited<ReturnType<typeof compileApplication>>>();
        const requested = new Map<
            string,
            {
                options: CompileOptions;
                application?: { name: string; options: ApplicationOptions };
                side?: "client" | "ssr";
            }
        >();

        // expand web applications into browser and server outputs
        for (const [name, selected] of Object.entries(options.outputs)) {
            const output =
                selected.kind === "web" ? await resolveView(options.directory, selected) : selected;
            if (!/^[a-z][a-z0-9-]*$/.test(name)) {
                throw new BuildError("BUILD_FAILED", `Invalid output name: ${name}`);
            }

            // expand a web application into its runtime environments
            if (output.kind === "web") {
                if (output.prerender && output.ssr === false) {
                    throw new BuildError("BUILD_FAILED", "Prerendering requires an SSR handler.");
                }

                // reserve generated output names
                const application = { name, options: output };
                for (const generated of [`${name}-browser`, `${name}-server`]) {
                    if (Object.hasOwn(options.outputs, generated)) {
                        throw new BuildError("BUILD_FAILED", `Duplicate output name: ${generated}`);
                    }
                }

                // always compile the browser entry
                requested.set(`${name}-browser`, {
                    options: {
                        kind: "module",
                        target: "browser",
                        runtime: "browser",
                        entries: { ".": output.app ?? "src/app.tsx" },
                        minify: output.minify,
                        base: output.base,
                    },
                    application,
                    side: "client",
                });

                // inspect the renderer even when only static pages are distributed
                if (output.ssr !== false) {
                    requested.set(`${name}-server`, {
                        options: {
                            kind: "module",
                            target: "server",
                            runtime: output.ssr.runtime,
                            entries: { ".": output.app ?? "src/app.tsx" },
                            minify: output.minify,
                            base: output.base,
                        },
                        application,
                        side: "ssr",
                    });
                }
            } else {
                if (requested.has(name)) {
                    throw new BuildError("BUILD_FAILED", `Duplicate output name: ${name}`);
                }
                requested.set(name, { options: output });
            }
        }

        // require at least one output
        if (!requested.size) {
            throw new BuildError("BUILD_FAILED", "A build requires at least one output.");
        }

        // inspect every target before framework compilation builds both browser and server code
        const inputs = new Map<
            string,
            { project: PackageSource; inspection: TypeScriptInspection }
        >();
        const inspections = new Map<string, TypeScriptInspection>();
        const evaluated = new Map<TypeScriptInspection, DeclarationDescription[]>();
        for (const [name, request] of requested) {
            const output = request.options;
            const application = request.application?.options;
            const frameworkFiles = application
                ? [
                      application.document,
                      application.entryClient,
                      application.entryServer,
                      application.middleware,
                      application.setup,
                      application.renderMode &&
                      !["sync", "async", "stream"].includes(application.renderMode)
                          ? application.renderMode
                          : undefined,
                  ].filter((path): path is string => typeof path === "string")
                : [];
            // include framework entries in the compiler roots
            const project = await this.open({
                directory: options.directory,
                configuration: options.configuration,
                target: output.target,
                runtime: output.runtime,
                entries: output.entries,
                files: frameworkFiles,
            });

            // reuse identical inspections within this build
            const key = project.configuration;
            let inspection = inspections.get(key);
            if (!inspection) {
                inspection = await this.typescript.inspect(project.configuration);
                inspections.set(key, inspection);
            }
            inputs.set(name, { project, inspection });
        }

        // reject source diagnostics before compiling publication outputs
        const check = {
            directory: options.directory,
            files: [
                ...new Set(
                    [...inputs.values()].flatMap(({ inspection }) => [
                        ...inspection.sources.keys(),
                    ]),
                ),
            ],
            signal: options.signal,
        };
        const checked = await checkPackage(check);
        if (checked.diagnostics.length) {
            throw new BuildError("BUILD_FAILED", JSON.stringify(checked));
        }

        // require canonical formatting without changing source
        const formatted = await formatPackage(check, false);
        if (formatted.code !== 0) {
            throw new BuildError("BUILD_FAILED", formatted.stdout + formatted.stderr);
        }

        // retain every inspected environment before the web compiler starts either environment
        for (const { inspection } of inputs.values()) {
            for (const [path, bytes] of inspection.sources) {
                retainFile(path, bytes, files);
            }
        }

        // share equal descriptions while retaining target-specific variants
        const moduleDescriptions: ManifestDescription["modules"] = [];
        const declarationDescriptions: ManifestDescription["declarations"] = [];
        const testDescriptions: ManifestDescription["tests"] = [];
        const selections: ManifestDescription["selections"] = new Map();
        const resolved: Record<string, DependencyResolution> = {};
        const moduleIndices = new Map<string, number>();
        const declarationIndices = new Map<string, number>();
        const testIndices = new Map<string, number>();

        // describe declarations and compile each selected output
        for (const [name, request] of requested) {
            const output = request.options;
            const { project, inspection: inspected } = inputs.get(name)!;
            if (
                source &&
                (source.id !== project.declaration.package.id ||
                    source.name !== project.declaration.package.name ||
                    source.version !== project.declaration.package.version)
            ) {
                throw new BuildError("BUILD_FAILED", "Package changed during build.");
            }
            source = project.declaration.package;

            // retain template assets alongside inspected source modules
            if (project.declaration.definition.template) {
                const template = await Template.read(project.directory);
                for (const [path, bytes] of template.files) {
                    retainFile(path, new Uint8Array(bytes), files);
                }
            }

            // retain authored package declarations
            for (const path of ["package.json", "destack.json"]) {
                retainFile(
                    path,
                    new Uint8Array(await readFile(resolve(project.directory, path))),
                    files,
                );
            }

            // evaluate each collected declaration set once
            const { modules, directories } = inspected;
            let declarations = evaluated.get(inspected);
            if (!declarations) {
                declarations = await evaluateDeclarations(inspected.declarations, project);
                evaluated.set(inspected, declarations);
            }

            // compile each output from the retained source bytes
            let buildDescription: BuildDescription;
            if (request.application) {
                const { name: applicationName, options: applicationOptions } = request.application;
                let application = applications.get(applicationName);
                if (!application) {
                    application = await compileApplication(
                        applicationName,
                        applicationOptions,
                        output,
                        applicationOptions.ssr === false
                            ? undefined
                            : {
                                  kind: "module",
                                  target: "server",
                                  runtime: applicationOptions.ssr.runtime,
                                  minify: applicationOptions.minify,
                              },
                        files,
                        project,
                        dependencies,
                        new Map([
                            ...directories,
                            ...(inputs.get(`${applicationName}-server`)?.inspection.directories ??
                                []),
                        ]),
                        inputs.get(`${applicationName}-server`)?.inspection.modules ?? [],
                        this.runtimes,
                        destination,
                    );
                    applications.set(applicationName, application);
                    for (const [path, bytes] of application.files) {
                        await writeBuildFile(path, bytes, destination, records);
                    }
                    application.files.clear();
                    for (const path of application.paths) {
                        await describeBuildFile(path, destination, records);
                    }
                    sourceMaps.push(...application.sourceMaps);
                }

                // associate this environment with its compiled files and inspection
                const compiled = application.outputs[request.side!];
                outputs[name] = compiled;
                buildDescription = application.inspections[request.side!];
            } else {
                const compiled = await compilePackage(
                    name,
                    output,
                    dependencies,
                    files,
                    project,
                    directories,
                    destination,
                );

                // retain compiled modules and their source maps
                for (const [path, bytes] of compiled.files) {
                    await writeBuildFile(path, bytes, destination, records);
                }
                compiled.files.clear();
                for (const path of compiled.paths) {
                    await describeBuildFile(path, destination, records);
                }
                outputs[name] = compiled.output;
                sourceMaps.push(...compiled.sourceMaps);
                buildDescription = compiled.inspection;
            }

            // validate workload declarations against the compiled output
            const selected = selectDeclarations(declarations, source, buildDescription);
            for (const declaration of selected) {
                const owner = declaration.symbol.package;
                if (owner.id === source.id && owner.version === source.version) {
                    continue;
                }

                // require the declaration package to be present at its exact version
                const key = `${owner.name}@${owner.version}`;
                const dependency =
                    buildDescription.packages[key] ?? outputs[name].dependencies[owner.name];
                if (
                    !dependency ||
                    dependency.package.version !== owner.version ||
                    (dependency.kind !== "npm" && dependency.package.id !== owner.id)
                ) {
                    throw new BuildError("BUILD_FAILED", `Unresolved declaration package: ${key}`);
                }
                if (
                    dependency.kind === "source" &&
                    !dependency.files.some((file) => file.path === declaration.source.file)
                ) {
                    throw new BuildError(
                        "BUILD_FAILED",
                        `Missing declaration source: ${key}/${declaration.source.file}`,
                    );
                }
            }

            // check the runtime and associate runnable workloads
            if (!request.application || project.runtime === "browser") {
                await checkRuntime(buildDescription, modules, project.runtime, this.runtimes);
            }
            outputs[name].workloads = await describeWorkloads(
                selected,
                modules,
                project,
                outputs[name].exports,
                buildDescription,
                request.side === "ssr",
            );

            // retain exact dependency resolutions once
            for (const [key, dependency] of Object.entries(buildDescription.packages)) {
                if (resolved[key] && JSON.stringify(resolved[key]) !== JSON.stringify(dependency)) {
                    throw new BuildError(
                        "BUILD_FAILED",
                        `conflicting dependency resolution: ${key}`,
                    );
                }
                resolved[key] = dependency;
            }
            selections.set(name, {
                modules: inspected.modules.map((value) =>
                    retainDescription(value, moduleDescriptions, moduleIndices),
                ),
                declarations: selected.map((value) =>
                    retainDescription(value, declarationDescriptions, declarationIndices),
                ),
                tests: inspected.tests.map((value) =>
                    retainDescription(value, testDescriptions, testIndices),
                ),
            });
        }

        // retain build-only renderer descriptions without deployment entrypoints
        for (const [name, request] of requested) {
            const application = request.application?.options;
            if (
                request.side === "ssr" &&
                application?.ssr !== false &&
                application?.ssr.emit === false
            ) {
                outputs[name].exports = {};
                outputs[name].emit = false;
                outputs[name].workloads = {};
            }
        }

        // serialize shared descriptions by domain and module
        const manifest = await serializeDescriptions(
            {
                modules: moduleDescriptions,
                declarations: declarationDescriptions,
                tests: testDescriptions,
                modulePackage: await findPackage("@destack/package"),
                testPackage: await findPackage("@destack/test"),
                selections,
            },
            outputs,
        );

        // describe distributed files in a stable order
        for (const [path, bytes] of files) {
            await writeBuildFile(path, bytes, destination, records);
        }
        files.clear();
        const descriptions = [];
        const ordered = [...records.values()].sort((left, right) =>
            left.path < right.path ? -1 : left.path > right.path ? 1 : 0,
        );
        for (const file of ordered) {
            const inspected = manifest.modules.get(file.path);
            descriptions.push(inspected ? { ...file, descriptions: inspected } : file);
        }

        // require every inspected path to identify a distributed file
        for (const path of manifest.modules.keys()) {
            if (!records.has(path)) {
                throw new BuildError(
                    "BUILD_FAILED",
                    `inspected file is absent from build: ${path}`,
                );
            }
        }

        // publish inventories separately from source and executable files
        const inventories = { dependencies: resolved, files: descriptions, sourceMaps };
        const references = {} as Pick<PackageManifest, "dependencies" | "files" | "sourceMaps">;
        for (const name of ["dependencies", "files", "sourceMaps"] as const) {
            const path = `manifest/${name}.json`;
            const bytes = encodeDescription(inventories[name]);
            references[name] = await describeFile(path, "application/json", bytes);
            await writeBuildFile(path, bytes, destination, records);
        }
        for (const [path, bytes] of manifest.files) {
            await writeBuildFile(path, bytes, destination, records);
        }

        // assemble the manifest
        const result: PackageManifest = {
            formatVersion: 1,
            package: source!,
            language: "typescript",
            dependencies: references.dependencies,
            descriptions: manifest.descriptions,
            outputs,
            files: references.files,
            sourceMaps: references.sourceMaps,
        };
        await writeFile(join(destination, "manifest.json"), JSON.stringify(result), { flag: "wx" });

        return new PackageBuild(result, destination, "retained");
    }

    /** Reuse a configuration until its package declarations change. */
    async open(options: Parameters<typeof openPackage>[0]): Promise<PackageSource> {
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
        const source = await openPackage(options);
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
}

/** Share equal descriptions and return their manifest index. */
function retainDescription<Value>(
    value: Value,
    values: Value[],
    indices: Map<string, number>,
): number {
    // reuse the index of an equal value
    const key = stringifyInspection(value);
    const existing = indices.get(key);
    if (existing !== undefined) {
        return existing;
    }

    // retain each target-specific variant once
    const index = values.length;
    values.push(value);
    indices.set(key, index);

    return index;
}

/** Retain one file and reject changes between output builds. */
function retainFile(
    path: string,
    bytes: Uint8Array<ArrayBuffer>,
    files: Map<string, Uint8Array<ArrayBuffer>>,
): void {
    const previous = files.get(path);
    if (
        previous &&
        (previous.length !== bytes.length ||
            previous.some((value, index) => value !== bytes[index]))
    ) {
        throw new BuildError("BUILD_FAILED", `Conflicting build file: ${path}`);
    }

    files.set(path, bytes);
}

/** Write one output and retain only its authenticated file record. */
async function writeBuildFile(
    path: string,
    bytes: Uint8Array<ArrayBuffer>,
    directory: string,
    records: Map<string, PackageFile>,
): Promise<void> {
    // preserve media types used by the distributed manifest
    const file = await describeFile(path, fileMediaType(path), bytes);
    const previous = records.get(path);
    if (previous) {
        if (previous.digest !== file.digest) {
            throw new BuildError("BUILD_FAILED", `conflicting build file: ${path}`);
        }
        return;
    }

    // publish bytes before recording the successful write
    const destination = join(directory, path);
    await mkdir(dirname(destination), { recursive: true });
    await writeFile(destination, bytes, { flag: "wx" });
    records.set(path, file);
}

/** Hash a compiler-written output without reading the entire file into memory. */
async function describeBuildFile(
    path: string,
    directory: string,
    records: Map<string, PackageFile>,
): Promise<void> {
    // hash and measure the file
    const hash = createHash("sha256");
    let size = 0;
    for await (const bytes of createReadStream(join(directory, path))) {
        hash.update(bytes);
        size += bytes.length;
    }
    const digest = hash.digest("hex");
    const previous = records.get(path);
    if (previous && previous.digest !== digest) {
        throw new BuildError("BUILD_FAILED", `conflicting build file: ${path}`);
    }
    records.set(path, { path, digest, size, mediaType: fileMediaType(path) });
}

/** Select the distributed media type from a generated path. */
function fileMediaType(path: string): string {
    const extension = path.split(".").at(-1);

    return extension === "json" || extension === "map"
        ? "application/json"
        : extension === "js"
          ? "text/javascript"
          : extension === "html"
            ? "text/html"
            : extension === "css"
              ? "text/css"
              : "application/octet-stream";
}

/** Read the identity of a Destack package resolved from the build tool's dependencies. */
async function findPackage(specifier: string): Promise<Package> {
    const owner = await new ModulePackages().find(fileURLToPath(import.meta.resolve(specifier)));
    if (!owner) {
        throw new BuildError("BUILD_FAILED", `missing Destack package: ${specifier}`);
    }

    return owner.metadata.package;
}
