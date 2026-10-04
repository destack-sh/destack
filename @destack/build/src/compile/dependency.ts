import { readFile } from "node:fs/promises";
import { dirname, isAbsolute, join, resolve, sep, posix } from "node:path";
import { type Plugin } from "vite";
import { type DependencyResolution } from "@destack/package";
import { comparePath, compareText } from "../build/serialization.ts";
import { BuildError, isMissing } from "../error/index.ts";
import { type PackageSource } from "../source/index.ts";
import { PackageFile } from "@destack/package/file";
import { BuildDescription } from "@destack/package/inspect";
import { DependencyName, Package, PackageDefinition } from "@destack/package";
import { type Runtime } from "@destack/package/runtime";
import { modulePackage, relativePath } from "../source/dependency.ts";
import { isBuiltin } from "node:module";
import { type OutputBundle, type OutputChunk, RUNTIME_MODULE_ID } from "rolldown";
import { type ESTree, Visitor } from "rolldown/utils";
import { found } from "@destack/schema";
import { Catalog } from "@destack/locale";
import { readCatalogs } from "../build/catalog.ts";

/** A parsed module's source package and path. */
export interface ModuleSource {
    /** Whether the module came from a file or a compiler plugin. */
    kind: "source" | "virtual";
    /** The dependency resolution key. */
    package?: string;
    /** The package-relative source path or virtual identifier. */
    path: string;
    /** Reviewed runtimes for this resolved module. */
    runtimes?: Runtime[];
    /** Import expressions requiring execution to resolve. */
    unresolvedImports?: string[];
}

/** A parsed module's resolved imports. */
export interface ParsedImports {
    /** Statically imported module identifiers. */
    importedIds: string[];
    /** Dynamically imported module identifiers. */
    dynamicallyImportedIds: string[];
}

/** One import of a described input or output. */
interface DescribedImport {
    /** The imported module's identifier, or the external specifier. */
    readonly path: string;
    /** Whether the import leaves the compilation. */
    readonly external: boolean;
    /** Whether the import is dynamic. */
    readonly dynamic: boolean;
}

/** A package containing modules, with the name and version its package.json declares. */
type PackageLocation = Awaited<ReturnType<typeof modulePackage>>;

/** The source package snapshot a build keeps of an unpublished dependency. */
type SourceSnapshot = Extract<DependencyResolution, { kind: "source" }>;

/** Package manifests reused by dependency resolution and output inspection. */
class DependencyResolver {
    /** Package owners indexed by source directory. */
    readonly #packages = new Map<string, Promise<PackageLocation>>();
    /** Destack declarations indexed by package directory. */
    readonly #definitions = new Map<string, Promise<PackageDefinition | undefined>>();

    /** Locate the manifest that contains a resolved module. */
    package(directory: string): Promise<PackageLocation> {
        let result = this.#packages.get(directory);
        if (!result) {
            result = modulePackage(directory);
            this.#packages.set(directory, result);
        }

        return result;
    }

    /** Read the optional Destack declaration once per compilation. */
    definition(directory: string): Promise<PackageDefinition | undefined> {
        let result = this.#definitions.get(directory);
        if (!result) {
            result = readPackageDefinition(directory);
            this.#definitions.set(directory, result);
        }

        return result;
    }
}

/** The packages, sources and assets one compilation reads, recorded into its description. */
class DependencyRecorder {
    /** The compiled package. */
    readonly project: PackageSource;
    /** The dependency releases the build resolves against. */
    readonly resolutions: Readonly<Record<string, DependencyResolution>>;
    /** The description the compilation fills. */
    readonly description: BuildDescription;
    /** The authored location of each compiled module. */
    readonly locations: Map<string, ModuleSource>;
    /** The assets read beside modules, by path. */
    readonly assets: ReadonlyMap<string, Uint8Array<ArrayBuffer>>;
    /** The build files the compilation writes, which keep the catalogs of unpublished dependencies. */
    readonly files: Map<string, Uint8Array<ArrayBuffer>>;
    /** The package manifests read so far. */
    readonly resolver = new DependencyResolver();
    /** The directory of each unpublished source dependency, by release. */
    readonly #directories = new Map<string, string>();

    /** Record into a compilation's description. */
    constructor(
        project: PackageSource,
        resolutions: Readonly<Record<string, DependencyResolution>>,
        description: BuildDescription,
        locations: Map<string, ModuleSource>,
        assets: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
    ) {
        // keep the compiled package, the releases and what the compilation fills
        this.project = project;
        this.resolutions = resolutions;
        this.description = description;
        this.locations = locations;
        this.assets = assets;
    }

    /** Record a parsed module's location and the package it comes from. */
    async parsed(
        id: string,
        runtimes: Runtime[] | undefined,
        parse: () => ESTree.Program,
    ): Promise<void> {
        // record a virtual module, which has no package declaration on disk
        const path = modulePath(id);
        if (!isAbsolute(path)) {
            this.locations.set(id, {
                kind: "virtual",
                path: id.replaceAll(this.project.directory, "."),
            });

            return;
        }

        // index physical asset paths used without import queries by Vite's manifest
        const source = await this.resolver.package(dirname(path));
        const local = relativePath(source.directory, path);
        const query = id.slice(path.length);
        const isProject = source.directory === this.project.directory;
        const key = `${source.name}@${source.version}`;
        if (query) {
            this.locations.set(path, {
                kind: "source",
                path: local,
                ...(isProject ? {} : { package: key }),
            });
        }

        // record project sources with their runtimes and unresolved imports
        if (isProject) {
            const declared = this.project.declaration.definition.runtimes;
            this.locations.set(id, {
                kind: "source",
                path: `${local}${query}`,
                ...(declared === undefined ? {} : { runtimes: declared }),
                unresolvedImports: unresolvedImports(parse()),
            });
        }
        // record a dependency's module under its release
        else {
            const definition = await this.resolver.definition(source.directory);
            const reviewed = runtimes ?? definition?.runtimes;
            this.locations.set(id, {
                kind: "source",
                package: key,
                path: `${local}${query}`,
                ...(reviewed === undefined ? {} : { runtimes: reviewed }),
            });
            await this.#dependency(source, definition, path, key);
        }
    }

    /** Record an asset a bundle emits from an original file, keeping a copied directory asset in its package's snapshot. */
    async asset(root: string, original: string): Promise<void> {
        // attribute assets read by CSS plugins without creating JavaScript modules
        const absolute = resolve(root, original);
        const owner = await modulePackage(dirname(absolute));
        const path = relativePath(owner.directory, absolute);
        const key = `${owner.name}@${owner.version}`;
        const isExternal = owner.directory !== this.project.directory;
        if (!this.locations.has(absolute)) {
            this.locations.set(absolute, {
                kind: "source",
                path,
                ...(isExternal ? { package: key } : {}),
            });
        }
        if (isExternal && !this.description.packages[key]) {
            this.description.packages[key] = this.#release(
                owner,
                key,
                "unresolved asset dependency",
            );
        }

        // retain copied directory assets in their source package snapshot
        const bytes = this.assets.get(original);
        const snapshot = this.description.packages[key];
        if (bytes === undefined || snapshot === undefined || snapshot.kind !== "source") {
            return;
        }
        const file = await PackageFile.describe(path, "application/octet-stream", bytes);
        addFile(snapshot, file);
    }

    /** Record a dependency module: an unpublished source by its files, an installed one by its release. */
    async #dependency(
        source: PackageLocation,
        definition: PackageDefinition | undefined,
        path: string,
        key: string,
    ): Promise<void> {
        // match installed code to the resolver's immutable release
        const resolution = this.resolutions[key] ?? this.resolutions[source.name];
        if (resolution || path.split(sep).includes("node_modules")) {
            this.description.packages[key] = this.#release(
                source,
                key,
                "unresolved bundled dependency",
            );

            return;
        }

        // refuse two sources of one unpublished release
        const previous = this.#directories.get(key);
        if (previous !== undefined && previous !== source.directory) {
            throw new BuildError("BUILD_FAILED", `conflicting dependency sources: ${key}`);
        }
        this.#directories.set(key, source.directory);

        // identify authored modules independently of generated JavaScript
        const local = relativePath(source.directory, path);
        const bytes = new Uint8Array(await readFile(path));
        const file = await PackageFile.describe(local, "application/octet-stream", bytes);
        addFile(await this.#snapshot(source, definition, key), file);
    }

    /** Read an unpublished dependency's snapshot, starting it with the manifests that control its exports. */
    async #snapshot(
        source: PackageLocation,
        definition: PackageDefinition | undefined,
        key: string,
    ): Promise<SourceSnapshot> {
        // start the snapshot with the package's manifests
        let snapshot = this.description.packages[key];
        if (!snapshot) {
            const identity = { id: definition?.id, name: source.name, version: source.version };
            const created: SourceSnapshot = {
                kind: "source",
                package: Package.parse(identity),
                files: [],
            };
            this.description.packages[key] = created;
            for (const name of ["package.json", "destack.json"]) {
                const bytes = new Uint8Array(await readFile(join(source.directory, name)));
                addFile(created, await PackageFile.describe(name, "application/json", bytes));
            }
            await this.#keepCatalogs(created, source.directory);
            snapshot = created;
        }

        // refuse a release resolved otherwise
        if (snapshot.kind !== "source") {
            throw new BuildError("BUILD_FAILED", `conflicting dependency resolution: ${key}`);
        }

        return snapshot;
    }

    /** Keep an unpublished dependency's catalogs in its snapshot, and their bytes in the build below its package. */
    async #keepCatalogs(snapshot: SourceSnapshot, directory: string): Promise<void> {
        const owner = this.project.declaration.package.id;
        for (const [path, bytes] of await readCatalogs(directory, snapshot.package.id)) {
            // describe the catalog in the snapshot, and write it where views of the build read it
            addFile(snapshot, await PackageFile.describe(path, "application/json", bytes));
            const catalog = Catalog.parse(JSON.parse(new TextDecoder().decode(bytes)));
            this.files.set(Catalog.path(catalog, owner), bytes);
        }
    }

    /** Require the selected release of an installed package. */
    #release(owner: PackageLocation, key: string, failure: string): DependencyResolution {
        const resolution = this.resolutions[key] ?? this.resolutions[owner.name];
        if (
            !resolution ||
            resolution.package.name !== owner.name ||
            resolution.package.version !== owner.version
        ) {
            throw new BuildError("BUILD_FAILED", `${failure}: ${key}`);
        }

        return resolution;
    }
}

/** Add a file to a source snapshot in path order once, refusing other bytes at its path. */
function addFile(snapshot: SourceSnapshot, file: PackageFile): void {
    // keep an equal file once, and refuse a file whose bytes changed during the build
    const previous = snapshot.files.find((candidate) => candidate.path === file.path);
    if (previous !== undefined && previous.digest !== file.digest) {
        throw new BuildError("BUILD_FAILED", `dependency file changed during build: ${file.path}`);
    }
    if (previous !== undefined) {
        return;
    }

    // insert the file in path order
    snapshot.files.push(file);
    snapshot.files.sort(comparePath);
}

/** Record registry releases and local sources consumed by the bundler. */
export function dependencyPlugin(
    project: PackageSource,
    resolutions: Readonly<Record<string, DependencyResolution>>,
    description: BuildDescription,
    output: string,
    environment?: string,
    locations = new Map<string, ModuleSource>(),
    assets: ReadonlyMap<string, Uint8Array<ArrayBuffer>> = new Map(),
): Plugin {
    // track resolved packages, runtimes and imports across hooks
    const recorder = new DependencyRecorder(project, resolutions, description, locations, assets);
    const runtimes = new Map<string, Runtime[]>();
    const imports = new Map<string, ParsedImports>();
    let root: string;

    return {
        ...resolutionPlugin(project, resolutions, undefined, runtimes, recorder.resolver),
        name: "destack-dependencies",
        applyToEnvironment(candidate) {
            return environment === undefined || candidate.name === environment;
        },
        configResolved(configuration) {
            root = configuration.root;
        },
        async moduleParsed(module) {
            // retain resolved imports while Rolldown already supplies the module
            imports.set(module.id, {
                importedIds: module.importedIds,
                dynamicallyImportedIds: module.dynamicallyImportedIds,
            });

            // record the module's location and package
            const code = module.code;
            if (code === null) {
                throw new BuildError("BUILD_FAILED", `parsed module has no source: ${module.id}`);
            }
            await recorder.parsed(module.id, runtimes.get(module.id), () => this.parse(code));
        },
        async generateBundle(_, bundle) {
            // identify retained directory assets alongside the dependency's source modules
            for (const entry of Object.values(bundle)) {
                if (entry.type === "asset") {
                    for (const original of entry.originalFileNames) {
                        await recorder.asset(root, original);
                    }
                }
            }

            // describe the compiled inputs and outputs
            const inspection = describeCompilation(
                locations,
                description.packages,
                output,
                bundle,
                imports,
            );
            description.packages = inspection.packages;
            description.inputs = inspection.inputs;
            description.outputs = inspection.outputs;
        },
    };
}

/** Read a package's Destack declaration when it provides one. */
async function readPackageDefinition(directory: string): Promise<PackageDefinition | undefined> {
    try {
        const text = await readFile(join(directory, "destack.json"), "utf8");

        return PackageDefinition.parse(JSON.parse(text));
    } catch (error) {
        if (isMissing(error)) {
            return undefined;
        }
        throw error;
    }
}

/** Resolve installed dependencies under the same rules for builds and previews. */
export function resolutionPlugin(
    source: PackageSource,
    dependencies: Readonly<Record<string, DependencyResolution>>,
    runtime?: Runtime,
    runtimes = new Map<string, Runtime[]>(),
    resolver = new DependencyResolver(),
): Plugin {
    return {
        name: "destack-resolution",
        resolveId: {
            filter: { id: { exclude: [/^(?:\.|\/|\0|#|virtual:|node:)/u] } },
            async handler(specifier, importer) {
                if (importer === undefined || isBuiltin(specifier)) {
                    return null;
                }
                const name = DependencyName.of(specifier);

                // let Vite apply package exports and the selected environment conditions
                const resolved = await this.resolve(specifier, importer, { skipSelf: true });
                if (resolved === null || resolved.external !== false || !isAbsolute(resolved.id)) {
                    return resolved;
                }

                // require authored imports to appear in the package manifest
                requireDeclared(source, importer, name);

                // reject installed releases that differ from the host's selected dependencies
                const path = modulePath(resolved.id);
                const owner = await resolver.package(dirname(path));
                requireSelected(source, dependencies, owner, name, path);

                // enforce compatibility for the exact imported Destack export
                const declared = await resolver.definition(owner.directory);
                const entry = specifier === name ? "." : `.${specifier.slice(name.length)}`;
                const required = declared && PackageDefinition.runtimes(declared, entry);
                if (required) {
                    const previous = runtimes.get(resolved.id);
                    const narrowed = previous?.filter((candidate) => required.includes(candidate));
                    runtimes.set(resolved.id, narrowed ?? required);
                }
                const selected =
                    runtime && (this.environment.name === "client" ? "browser" : runtime);
                if (selected && required && !required.includes(selected)) {
                    throw new BuildError(
                        "BUILD_FAILED",
                        `unsupported ${selected} dependency: ${specifier}`,
                    );
                }

                return resolved;
            },
        },
    };
}

/** Require an authored module's import to name the package or a dependency its manifest declares. */
function requireDeclared(source: PackageSource, importer: string, name: string): void {
    // leave imports of dependency modules to the dependencies' manifests
    const local = relativePath(source.directory, importer);
    const isAuthored =
        isAbsolute(importer) &&
        !local.startsWith("../") &&
        !isAbsolute(local) &&
        !local.split("/").includes("node_modules");
    const declaration = source.declaration;
    if (!isAuthored || name === declaration.package.name) {
        return;
    }

    // refuse a dependency no requirement group names
    const groups = [
        declaration.dependencies,
        declaration.peerDependencies,
        declaration.optionalDependencies,
    ];
    if (!groups.some((requirements) => Object.hasOwn(requirements, name))) {
        throw new BuildError("BUILD_FAILED", `undeclared runtime dependency: ${name}`);
    }
}

/** Require an installed dependency to be the release the host selected. */
function requireSelected(
    source: PackageSource,
    dependencies: Readonly<Record<string, DependencyResolution>>,
    owner: PackageLocation,
    name: string,
    path: string,
): void {
    // accept the package's modules
    if (owner.directory === source.directory) {
        return;
    }

    // refuse a different release than the selected one
    const key = `${owner.name}@${owner.version}`;
    const selected = dependencies[key] ?? dependencies[name];
    if (selected) {
        if (selected.package.name !== owner.name || selected.package.version !== owner.version) {
            throw new BuildError(
                "BUILD_FAILED",
                `dependency resolution differs from installation: ${name}`,
            );
        }
    }
    // refuse an installed release without a selection
    else if (path.split(sep).includes("node_modules")) {
        throw new BuildError("BUILD_FAILED", `unresolved installed dependency: ${key}`);
    }
}

/** Describe resolved module imports and generated file membership. */
export function describeCompilation(
    locations: ReadonlyMap<string, ModuleSource>,
    packages: BuildDescription["packages"],
    output: string,
    bundle: OutputBundle,
    parsed: ReadonlyMap<string, ParsedImports>,
): BuildDescription {
    // describe parsed inputs, then each emitted file
    const description: BuildDescription = { packages, inputs: {}, outputs: {} };
    const identifiers = describeInputs(description, locations, parsed);
    for (const file of Object.values(bundle)) {
        const isChunk = file.type === "chunk";
        description.outputs[`${output}/${file.fileName}`] = {
            inputs: isChunk ? chunkInputs(description, file, identifiers).toSorted() : [],
            imports: isChunk ? chunkImports(file, bundle, output) : [],
            exports: isChunk ? file.exports : [],
        };
    }

    // stabilize package order after parallel module parsing
    description.packages = Object.fromEntries(
        Object.entries(description.packages).toSorted(([left], [right]) =>
            compareText(left, right),
        ),
    );

    return description;
}

/** Describe parsed inputs in stable source order, returning each module's identifier. */
function describeInputs(
    description: BuildDescription,
    locations: ReadonlyMap<string, ModuleSource>,
    parsed: ReadonlyMap<string, ParsedImports>,
): Map<string, string> {
    // order the parsed modules by kind, package and path
    const names = new Map<string, string>();
    for (const id of parsed.keys()) {
        const location = locations.get(id);
        if (!location) {
            throw new BuildError("BUILD_FAILED", `missing parsed module: ${id}`);
        }
        names.set(id, JSON.stringify([location.kind, location.package ?? "", location.path]));
    }
    const modules = [...parsed.keys()].toSorted((left, right) =>
        compareText(found(names, left), found(names, right)),
    );
    const identifiers = new Map(modules.map((id, index) => [id, `module:${index}`]));

    // describe each module with its imports
    for (const id of modules) {
        const module = found(parsed, id);
        const imports: DescribedImport[] = [];
        for (const [dynamic, paths] of [
            [false, module.importedIds],
            [true, module.dynamicallyImportedIds],
        ] as const) {
            for (const path of paths) {
                // name a parsed import by its identifier and an external one by its specifier
                const isExternal = !parsed.has(path);
                const reference = isExternal ? path : identifiers.get(path);
                if (reference === undefined) {
                    throw new BuildError("BUILD_FAILED", `unresolved module: ${path}`);
                }
                imports.push({ path: reference, external: isExternal, dynamic });
            }
        }
        description.inputs[found(identifiers, id)] = { ...found(locations, id), imports };
    }

    return identifiers;
}

/** List the inputs a chunk renders, describing Rolldown's runtime once it appears. */
function chunkInputs(
    description: BuildDescription,
    chunk: OutputChunk,
    identifiers: Map<string, string>,
): string[] {
    // materialize the native module map once per chunk
    const rendered = Object.entries(chunk.modules).toSorted(([left], [right]) =>
        compareText(left, right),
    );
    const inputs: string[] = [];
    for (const [id, module] of rendered) {
        if (module.renderedLength === 0) {
            continue;
        }

        // describe the runtime Rolldown adds after parsing authored and virtual inputs
        if (id === RUNTIME_MODULE_ID && !identifiers.has(id)) {
            const reference = "generated:rolldown/runtime";
            identifiers.set(id, reference);
            description.inputs[reference] = { kind: "generated", path: id, imports: [] };
        }
        const reference = identifiers.get(id);
        if (reference === undefined) {
            throw new BuildError("BUILD_FAILED", `unknown emitted module: ${id}`);
        }
        inputs.push(reference);
    }

    return inputs;
}

/** List a chunk's imports, naming the bundle's files by output path and others by specifier. */
function chunkImports(chunk: OutputChunk, bundle: OutputBundle, output: string): DescribedImport[] {
    const imports: DescribedImport[] = [];
    for (const [dynamic, paths] of [
        [false, chunk.imports],
        [true, chunk.dynamicImports],
    ] as const) {
        for (const imported of paths) {
            const internal = bundledFile(chunk, bundle, imported);
            imports.push({
                path: internal === undefined ? imported : `${output}/${internal}`,
                external: internal === undefined,
                dynamic,
            });
        }
    }

    return imports;
}

/** Find the bundle file a chunk imports, by its name or its path relative to the chunk. */
function bundledFile(
    chunk: OutputChunk,
    bundle: OutputBundle,
    imported: string,
): string | undefined {
    const resolved = posix.normalize(posix.join(posix.dirname(chunk.fileName), imported));

    // match the import as written
    if (Object.hasOwn(bundle, imported)) {
        return imported;
    }
    // match the import relative to the chunk
    else if (Object.hasOwn(bundle, resolved)) {
        return resolved;
    }
    // leave the import outside the bundle
    else {
        return undefined;
    }
}

/** Read a module identifier's file path, without its query. */
export function modulePath(id: string): string {
    return id.replace(/\?[\s\S]*$/u, "");
}

/** Find dynamic imports whose module names require application execution. */
export function unresolvedImports(ast: ESTree.Program): string[] {
    const imports: string[] = [];
    new Visitor({
        ImportExpression(node) {
            if (node.source.type !== "Literal" || typeof node.source.value !== "string") {
                imports.push(`import expression at offset ${node.start}`);
            }
        },
    }).visit(ast);

    return imports;
}
