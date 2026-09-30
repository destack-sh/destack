import { readFile } from "node:fs/promises";
import { isAbsolute, join, relative, resolve, sep } from "node:path";
import { build, type Plugin } from "vite";
import { DependencyRelease, type DependencyResolution, type Package } from "@destack/package";
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
import { PackagePath } from "@destack/package/file";
import type { BuildDescription, DeclarationDescription } from "@destack/package/inspect";
import { type PackageOutput } from "@destack/package/manifest";
import { type SourceMapReference } from "@destack/package/source";
import { modulePlugin } from "@destack/package/transform/vite";
import { type Runtime } from "@destack/package/runtime";
import { type PackageSource } from "../source/index.ts";
import { type DirectoryReference } from "../typescript/index.ts";
import { BuildError } from "../error/index.ts";
import { sourcePlugin, mapSource } from "./source.ts";
import { dependencyPlugin, type ModuleSource } from "./dependency.ts";
import { directoryPlugin } from "./asset.ts";

/** Compilation settings for a named package output. */
export interface ModuleOptions {
    /** Compile package modules. */
    kind: "module";
    /** The runtime the output is compiled for. */
    runtime: Runtime;
    /** Package-relative module or HTML entries; omit to compile package exports. */
    entries?: Readonly<Record<string, string>>;
    /** Bundle dependencies; disabled for separately installed library dependencies. */
    bundle?: boolean;
    /** Minify generated code with Vite's minifier. */
    minify?: boolean;
    /** Public URL prefix used by browser assets. */
    base?: string;
}

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

    /** The runtime the output runs on. */
    get runtime(): Runtime {
        return this.project.runtime;
    }

    /** Locate the module and export of one of the package's own declarations. */
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
        // collect output paths and files under the output directory
        const { options, project } = this;
        const directory = `output/${this.name}`;
        const destination = resolve(destinationRoot, directory);
        const paths: string[] = [];
        const files = new Map<string, Uint8Array<ArrayBuffer>>();
        const exports: Record<string, string> = {};
        const facades = new Map<string, string>();
        const external: Record<string, DependencyRelease> = {};
        const inspection: BuildDescription = { packages: {}, inputs: {}, outputs: {} };
        const locations = new Map<string, ModuleSource>();
        const assets = new Map<string, Uint8Array<ArrayBuffer>>();
        const sourceMaps: SourceMapReference[] = [];
        const isBrowser = project.runtime === "browser";

        // collect the extensions' plugins and entries before compiling
        const plugins = this.extensions.flatMap((extension) => {
            try {
                return extension.compile?.(this) ?? [];
            } catch (cause) {
                throw BuildError.from(cause);
            }
        });
        const entries = Object.values(project.exports);
        const output: PackageOutput = {
            runtime: project.runtime,
            emit: true,
            workloads: {},
            views: {},
            descriptions: {},
            tests: [],
            directory,
            exports,
            dependencies: external,
        };
        if (!entries.length && !this.#entries.size) {
            return { inspection, files, paths, sourceMaps, output };
        }
        if (entries.some((entry) => entry.endsWith(".html")) && !isBrowser) {
            throw new BuildError("BUILD_FAILED", "entries in HTML require the browser runtime");
        }

        // retain HTML before Vite rewrites its module and asset references
        for (const entry of entries.filter((entry) => entry.endsWith(".html"))) {
            files.set(
                relative(project.directory, entry).split(sep).join("/"),
                new Uint8Array(await readFile(entry)),
            );
        }

        // delegate JSX, CSS, assets, and minification to the extensions and standard plugins
        const generated = await build({
            root: project.directory,
            configFile: false,
            envDir: false,
            publicDir: false,
            base: options.base ?? "./",
            logLevel: "silent",
            define: { "process.env.NODE_ENV": JSON.stringify("production") },
            resolve: {
                conditions: runtimeConditions(project.runtime),
            },
            ssr: {
                noExternal: true,
                target: project.runtime === "workerd" ? "webworker" : "node",
                resolve: { conditions: runtimeConditions(project.runtime) },
            },
            plugins: [
                directoryPlugin(project.directory, files, this.directories, assets),
                sourcePlugin(project.directory, files, sources, destinationRoot, paths),
                dependencyPlugin(
                    project,
                    dependencies,
                    inspection,
                    directory,
                    undefined,
                    locations,
                    assets,
                ),
                this.#entryPlugin(facades),
                ...plugins,
                modulePlugin(),
            ],
            build: {
                emitAssets: true,
                write: true,
                outDir: destination,
                emptyOutDir: false,
                ssr: !isBrowser,
                minify: options.minify ?? false,
                sourcemap: true,
                copyPublicDir: false,
                rolldownOptions: {
                    platform: project.runtime === "bun" ? "node" : "browser",
                    resolve: {
                        mainFields:
                            project.runtime === "bun"
                                ? ["module", "main"]
                                : ["browser", "module", "main"],
                        conditionNames: runtimeConditions(project.runtime),
                    },
                    cwd: project.directory,
                    experimental: { attachDebugInfo: "none" },
                    input: entries,
                    preserveEntrySignatures: "strict",
                    // ignore plugin timings
                    checks: { pluginTimings: false },
                    onwarn(warning) {
                        throw new BuildError("BUILD_FAILED", warning.message);
                    },
                    external: (specifier, importer) =>
                        this.#isExternal(specifier, importer, dependencies, external),
                    output: {
                        format: "esm",
                        entryFileNames: "[name]-[hash].js",
                        chunkFileNames: "[name]-[hash].js",
                        assetFileNames: "asset/[name]-[hash][extname]",
                        sourcemapPathTransform(source, map) {
                            const output = `${directory}/${relative(destination, map).split(sep).join("/")}`;

                            return mapSource(source, map, output, locations);
                        },
                    },
                },
            },
        }).catch((error: unknown) => {
            throw BuildError.fromBundle(error);
        });

        // retain generated paths and map authored entries to generated files
        if (!("output" in generated)) {
            throw new BuildError("BUILD_FAILED", "expected one Vite output");
        }
        for (const entry of generated.output) {
            const path = PackagePath.parse(`${directory}/${entry.fileName}`);
            paths.push(path);
            for (const [name, source] of Object.entries(project.exports)) {
                if (entry.type === "chunk" && entry.facadeModuleId === source) {
                    exports[name] = path;
                    facades.set(name, source);
                }
                if (
                    entry.type === "asset" &&
                    source.endsWith(".html") &&
                    entry.fileName === relative(project.directory, source).split(sep).join("/")
                ) {
                    exports[name] = path;
                }
            }
            if (entry.type === "chunk" && entry.sourcemapFileName) {
                sourceMaps.push({
                    generated: path,
                    map: `${directory}/${entry.sourcemapFileName}`,
                });
            }
        }

        // export each entry's emitted chunk under its entrypoint
        for (const entrypoint of this.#entries.keys()) {
            const facade = facades.get(entrypoint);
            const chunk = generated.output.find(
                (entry) => entry.type === "chunk" && entry.facadeModuleId === facade,
            );
            if (chunk === undefined) {
                throw new BuildError("BUILD_FAILED", `missing generated entry: ${entrypoint}`);
            }
            exports[entrypoint] = PackagePath.parse(`${directory}/${chunk.fileName}`);
        }
        for (const name of Object.keys(project.exports)) {
            if (!exports[name]) {
                throw new BuildError("BUILD_FAILED", `missing generated entry: ${name}`);
            }
        }

        // let each extension describe the compiled output's workloads and views
        const compiled: CompiledOutput = {
            exports,
            reach: (entrypoint) => this.#reach(entrypoint, exports, facades, inspection),
        };
        for (const extension of this.extensions) {
            const described = describe(extension, this, compiled);
            merge(output, described);
        }

        return { inspection, output, files, paths, sourceMaps };
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
                    const name = entrypoint.replace(/^\.\//, "").replaceAll("/", "-");
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
        external: Record<string, DependencyRelease>,
    ): boolean {
        // keep host modules the runtime supplies, refusing the rest before Vite emits browser stubs
        const { options, project } = this;
        if (isHostModule(specifier)) {
            if (!isRuntimeModule(specifier, project.runtime)) {
                throw new BuildError(
                    "BUILD_FAILED",
                    `host module is unavailable on ${project.runtime}: ${specifier}`,
                );
            }

            return true;
        }

        // bundle relative, absolute, virtual and package-internal modules
        if (
            specifier.startsWith(".") ||
            specifier.startsWith("/") ||
            specifier.startsWith("\0") ||
            specifier.startsWith("#")
        ) {
            return false;
        }
        const dependency = specifier.startsWith("@")
            ? specifier.split("/").slice(0, 2).join("/")
            : specifier.split("/")[0]!;
        if (dependency === project.declaration.package.name) {
            return false;
        }

        // bundle what bundled dependencies import
        if (
            options.bundle &&
            importer &&
            (importer.split(sep).includes("node_modules") ||
                relative(project.directory, importer).startsWith(`..${sep}`))
        ) {
            return false;
        }

        // require a declared dependency
        if (
            ![
                project.declaration.dependencies,
                project.declaration.peerDependencies,
                project.declaration.optionalDependencies,
            ].some((requirements) => Object.hasOwn(requirements, dependency))
        ) {
            throw new BuildError("BUILD_FAILED", `undeclared runtime dependency: ${dependency}`);
        }

        // bundle it, or keep its registry release external
        if (options.bundle) {
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
        external[dependency] = DependencyRelease.parse(selected);

        return true;
    }

    /** Select the declarations an entrypoint's modules declare, including declarations shaken out of its chunks. */
    #reach(
        entrypoint: string,
        exports: Readonly<Record<string, string>>,
        facades: ReadonlyMap<string, string>,
        inspection: BuildDescription,
    ): DeclarationDescription[] {
        // require an emitted entrypoint
        const emitted = exports[entrypoint];
        const facade = facades.get(entrypoint);
        if (emitted === undefined || facade === undefined) {
            throw new BuildError("BUILD_FAILED", `no emitted entrypoint: ${entrypoint}`);
        }

        // refuse external imports of the entry's chunks the runtime lacks
        const chunks = [emitted];
        const visited = new Set<string>();
        for (const path of chunks) {
            if (visited.has(path)) {
                continue;
            }
            visited.add(path);
            const chunk = inspection.outputs[path];
            if (!chunk) {
                throw new BuildError("BUILD_FAILED", `unknown emitted chunk: ${path}`);
            }
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

        // start from the entry's module as the dependency plugin records it
        const root = isAbsolute(facade)
            ? relative(this.directory, facade).split(sep).join("/")
            : facade.replaceAll(this.directory, ".");
        const sources = Object.entries(inspection.inputs)
            .filter(([, input]) => !input.package && input.path.split("?")[0] === root)
            .map(([id]) => id);
        if (!sources.length) {
            throw new BuildError("BUILD_FAILED", `missing input of entrypoint: ${entrypoint}`);
        }

        // index the source files each package's reachable modules read
        const reachable = new Set<string>();
        const paths = new Map<string, Set<string>>();
        for (const id of sources) {
            if (reachable.has(id)) {
                continue;
            }
            reachable.add(id);
            const input = inspection.inputs[id];
            if (!input) {
                throw new BuildError("BUILD_FAILED", `unknown source module: ${id}`);
            }
            const owner = input.package
                ? inspection.packages[input.package]?.package
                : this.package;
            if (!owner) {
                throw new BuildError("BUILD_FAILED", `unknown source package: ${input.package}`);
            }
            const key = `${owner.name}@${owner.version}`;
            const files = paths.get(key) ?? new Set<string>();
            files.add(input.path.split("?")[0]!);
            paths.set(key, files);
            for (const imported of input.imports) {
                if (!imported.external) {
                    sources.push(imported.path);
                }
            }
        }

        // select the declarations the reachable files declare
        return this.declarations.filter((declaration) => {
            const owner = declaration.symbol.package;

            return (
                paths.get(`${owner.name}@${owner.version}`)?.has(declaration.source.file) === true
            );
        });
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
        throw BuildError.from(cause);
    }
}

/** Add an extension's workloads and views to an output, refusing a name described twice. */
function merge(output: PackageOutput, described: OutputDescription): void {
    for (const [section, entries] of [
        ["workloads", described.workloads],
        ["views", described.views],
    ] as const) {
        for (const [name, description] of Object.entries(entries ?? {})) {
            if (Object.hasOwn(output[section], name)) {
                throw new BuildError("BUILD_FAILED", `duplicate ${section} entry: ${name}`);
            }
            (output[section] as Record<string, unknown>)[name] = description;
        }
    }
}
