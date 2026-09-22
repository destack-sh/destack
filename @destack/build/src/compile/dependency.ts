import { readFile } from "node:fs/promises";
import { dirname, isAbsolute, join, relative, resolve, sep, posix } from "node:path";
import { type Plugin } from "vite";
import { type DependencyResolution } from "@destack/package/package";
import { BuildError } from "../error/index.ts";
import { type PackageSource } from "../source/index.ts";
import { describeFile } from "@destack/package/file";
import { BuildDescription } from "@destack/package/inspect";
import { Package, PackageDefinition } from "@destack/package";
import { type Runtime } from "@destack/package/runtime";
import { modulePackage } from "../source/dependency.ts";
import { isBuiltin } from "node:module";
import { type OutputBundle, RUNTIME_MODULE_ID } from "rolldown";
import { type ESTree, Visitor } from "rolldown/utils";

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
export interface ModuleImports {
    /** Statically imported module identifiers. */
    importedIds: string[];
    /** Dynamically imported module identifiers. */
    dynamicallyImportedIds: string[];
}

/** Package manifests reused by dependency resolution and output inspection. */
class DependencyResolver {
    /** Package owners indexed by source directory. */
    readonly #packages = new Map<string, ReturnType<typeof modulePackage>>();
    /** Destack declarations indexed by package directory. */
    readonly #definitions = new Map<string, Promise<PackageDefinition | undefined>>();

    /** Locate the manifest that contains a resolved module. */
    package(directory: string) {
        let result = this.#packages.get(directory);
        if (!result) {
            result = modulePackage(directory);
            this.#packages.set(directory, result);
        }

        return result;
    }

    /** Read the optional Destack declaration once per compilation. */
    definition(directory: string) {
        let result = this.#definitions.get(directory);
        if (!result) {
            result = readPackageDefinition(directory);
            this.#definitions.set(directory, result);
        }

        return result;
    }
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
    const resolver = new DependencyResolver();
    const directories = new Map<string, string>();
    const runtimes = new Map<string, Runtime[]>();
    const imports = new Map<string, ModuleImports>();
    let root: string;

    return {
        ...resolutionPlugin(project, resolutions, undefined, runtimes, resolver),
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

            // virtual modules have no package declaration on disk
            const path = module.id.split("?")[0];
            if (module.code === null) {
                throw new BuildError("BUILD_FAILED", `Parsed module has no source: ${module.id}`);
            }
            if (!isAbsolute(path)) {
                locations.set(module.id, {
                    kind: "virtual",
                    path: module.id.replaceAll(project.directory, "."),
                });
                return;
            }
            const directory = dirname(path);
            const source = await resolver.package(directory);
            const definition = await resolver.definition(source.directory);
            const declared = definition?.runtimes;
            const localPath = relative(source.directory, path).split(sep).join("/");
            const query = module.id.slice(path.length);

            // index physical asset paths used without import queries by Vite's manifest
            if (query) {
                locations.set(path, {
                    kind: "source",
                    path: localPath,
                    ...(source.directory === project.directory
                        ? {}
                        : { package: `${source.name}@${source.version}` }),
                });
            }

            if (source.directory === project.directory) {
                const unresolved = unresolvedImports(this.parse(module.code));
                locations.set(module.id, {
                    kind: "source",
                    path: `${localPath}${query}`,
                    runtimes: project.declaration.definition.runtimes,
                    unresolvedImports: unresolved,
                });
                return;
            }

            // match installed code to the resolver's immutable release
            const key = `${source.name}@${source.version}`;
            locations.set(module.id, {
                kind: "source",
                package: key,
                path: `${localPath}${query}`,
                runtimes: runtimes.get(module.id) ?? declared,
            });
            const resolution = resolutions[key] ?? resolutions[source.name];
            if (!resolution && !path.split(sep).includes("node_modules")) {
                const previousDirectory = directories.get(key);
                if (previousDirectory && previousDirectory !== source.directory) {
                    throw new BuildError("BUILD_FAILED", `Conflicting dependency sources: ${key}`);
                }
                directories.set(key, source.directory);

                // retain the declarations that control local package exports and resolution
                let snapshot = description.packages[key];
                if (!snapshot) {
                    snapshot = {
                        kind: "source",
                        package: Package.parse({
                            id: definition?.id,
                            name: source.name,
                            version: source.version,
                        }),
                        files: [],
                    };
                    description.packages[key] = snapshot;
                    for (const name of ["package.json", "destack.json"]) {
                        snapshot.files.push(
                            await describeFile(
                                name,
                                "application/json",
                                new Uint8Array(await readFile(join(source.directory, name))),
                            ),
                        );
                    }
                }
                if (snapshot.kind !== "source") {
                    throw new BuildError(
                        "BUILD_FAILED",
                        `Conflicting dependency resolution: ${key}`,
                    );
                }

                // identify authored modules independently of generated JavaScript
                const local = relative(source.directory, path).split(sep).join("/");
                if (!snapshot.files.some((entry) => entry.path === local)) {
                    const file = await describeFile(
                        local,
                        "application/octet-stream",
                        new Uint8Array(await readFile(path)),
                    );
                    snapshot.files.push(file);
                }
                snapshot.files.sort((left, right) =>
                    left.path < right.path ? -1 : left.path > right.path ? 1 : 0,
                );
                return;
            }
            if (
                !resolution ||
                resolution.package.name !== source.name ||
                resolution.package.version !== source.version
            ) {
                throw new BuildError("BUILD_FAILED", `Unresolved bundled dependency: ${key}`);
            }
            description.packages[key] = resolution;
        },
        async generateBundle(_, bundle) {
            // identify retained directory assets alongside the dependency's source modules
            for (const entry of Object.values(bundle)) {
                if (entry.type !== "asset") {
                    continue;
                }
                for (const original of entry.originalFileNames) {
                    // attribute assets read by CSS plugins without creating JavaScript modules
                    const absolute = resolve(root, original);
                    const owner = await modulePackage(dirname(absolute));
                    const path = relative(owner.directory, absolute).split(sep).join("/");
                    const key = `${owner.name}@${owner.version}`;
                    const external = owner.directory !== project.directory;
                    if (!locations.has(absolute)) {
                        locations.set(absolute, {
                            kind: "source",
                            path,
                            ...(external ? { package: key } : {}),
                        });
                    }
                    if (external && !description.packages[key]) {
                        const resolution = resolutions[key] ?? resolutions[owner.name];
                        if (
                            !resolution ||
                            resolution.package.name !== owner.name ||
                            resolution.package.version !== owner.version
                        ) {
                            throw new BuildError(
                                "BUILD_FAILED",
                                `Unresolved asset dependency: ${key}`,
                            );
                        }
                        description.packages[key] = resolution;
                    }

                    // retain copied directory assets in their source package snapshot
                    const bytes = assets.get(original);
                    if (!bytes) {
                        continue;
                    }
                    const snapshot = description.packages[key];
                    if (!snapshot || snapshot.kind !== "source") {
                        continue;
                    }
                    const file = await describeFile(path, "application/octet-stream", bytes);
                    const previous = snapshot.files.find((entry) => entry.path === path);
                    if (previous && previous.digest !== file.digest) {
                        throw new BuildError(
                            "BUILD_FAILED",
                            `Dependency asset changed during build: ${path}`,
                        );
                    }
                    if (!previous) {
                        snapshot.files.push(file);
                    }
                    snapshot.files.sort((left, right) =>
                        left.path < right.path ? -1 : left.path > right.path ? 1 : 0,
                    );
                }
            }

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
        if (error instanceof Error && "code" in error && error.code === "ENOENT") {
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
            filter: { id: { exclude: [/^(?:\.|\/|\0|#|virtual:|node:)/] } },
            async handler(specifier, importer) {
                if (!importer || isBuiltin(specifier)) {
                    return null;
                }
                const name = specifier.startsWith("@")
                    ? specifier.split("/").slice(0, 2).join("/")
                    : specifier.split("/")[0];

                // let Vite apply package exports and the selected environment conditions
                const resolved = await this.resolve(specifier, importer, { skipSelf: true });
                if (!resolved || resolved.external || !isAbsolute(resolved.id)) {
                    return resolved;
                }
                // require authored imports to appear in the package manifest
                const local = relative(source.directory, importer).split(sep).join("/");
                const authored =
                    isAbsolute(importer) &&
                    !local.startsWith("../") &&
                    !isAbsolute(local) &&
                    !local.split("/").includes("node_modules");
                const declaration = source.declaration;
                if (
                    authored &&
                    name !== declaration.package.name &&
                    ![
                        declaration.dependencies,
                        declaration.peerDependencies,
                        declaration.optionalDependencies,
                    ].some((dependencies) => Object.hasOwn(dependencies, name))
                ) {
                    throw new BuildError("BUILD_FAILED", `Undeclared runtime dependency: ${name}`);
                }

                const path = resolved.id.split("?")[0];
                const directory = dirname(path);
                const owner = await resolver.package(directory);

                // reject installed releases that differ from the host's selected dependencies
                if (owner.directory !== source.directory) {
                    const key = `${owner.name}@${owner.version}`;
                    const selected = dependencies[key] ?? dependencies[name];
                    if (selected) {
                        if (
                            selected.package.name !== owner.name ||
                            selected.package.version !== owner.version
                        ) {
                            throw new BuildError(
                                "BUILD_FAILED",
                                `dependency resolution differs from installation: ${name}`,
                            );
                        }
                    } else if (path.split(sep).includes("node_modules")) {
                        throw new BuildError(
                            "BUILD_FAILED",
                            `unresolved installed dependency: ${key}`,
                        );
                    }
                }

                // enforce compatibility for the exact imported Destack export
                const declared = await resolver.definition(owner.directory);
                const entry = specifier === name ? "." : `.${specifier.slice(name.length)}`;
                const required = declared?.exports?.[entry]?.runtimes ?? declared?.runtimes;
                if (required) {
                    const previous = runtimes.get(resolved.id);
                    runtimes.set(
                        resolved.id,
                        previous
                            ? previous.filter((runtime) => required.includes(runtime))
                            : required,
                    );
                }
                const selectedRuntime =
                    runtime && (this.environment.name === "client" ? "browser" : runtime);
                if (selectedRuntime && required && !required.includes(selectedRuntime)) {
                    throw new BuildError(
                        "BUILD_FAILED",
                        `Unsupported ${selectedRuntime} dependency: ${specifier}`,
                    );
                }

                return resolved;
            },
        },
    };
}

/** Describe resolved module imports and generated file membership. */
export function describeCompilation(
    locations: ReadonlyMap<string, ModuleSource>,
    packages: BuildDescription["packages"],
    output: string,
    bundle: OutputBundle,
    information: ReadonlyMap<string, ModuleImports>,
): BuildDescription {
    const description: BuildDescription = { packages, inputs: {}, outputs: {} };

    // index parsed inputs in stable source order
    const modules = [...information.keys()];
    const names = new Map<string, string>();
    for (const id of modules) {
        const location = locations.get(id);
        if (!location) {
            throw new BuildError("BUILD_FAILED", `Missing parsed module: ${id}`);
        }
        names.set(id, JSON.stringify([location.kind, location.package ?? "", location.path]));
    }
    modules.sort((left, right) => {
        const first = names.get(left)!;
        const second = names.get(right)!;

        return first < second ? -1 : first > second ? 1 : 0;
    });
    const identifiers = new Map(modules.map((id, index) => [id, `module:${index}`]));
    for (const id of modules) {
        const module = information.get(id);
        const location = locations.get(id);
        if (!module || !location) {
            throw new BuildError("BUILD_FAILED", `Missing parsed module: ${id}`);
        }
        const imports = [];
        for (const [dynamic, paths] of [
            [false, module.importedIds],
            [true, module.dynamicallyImportedIds],
        ] as const) {
            for (const path of paths) {
                // external imports do not produce moduleParsed events
                const external = !information.has(path);
                const reference = external ? path : identifiers.get(path);
                if (!reference) {
                    throw new BuildError("BUILD_FAILED", `Unresolved module: ${path}`);
                }
                imports.push({ path: reference, external, dynamic });
            }
        }
        description.inputs[identifiers.get(id)!] = { ...location, imports };
    }

    // retain chunk membership and imports separately from parsed input dependencies
    for (const file of Object.values(bundle)) {
        const path = `${output}/${file.fileName}`;
        const inputs = [];
        const imports = [];
        if (file.type === "chunk") {
            // materialize the native module map once per chunk
            const modules = file.modules;
            for (const id of Object.keys(modules).sort()) {
                if (modules[id].renderedLength === 0) {
                    continue;
                }
                // Rolldown adds its runtime after parsing authored and virtual inputs
                if (id === RUNTIME_MODULE_ID && !identifiers.has(id)) {
                    const reference = "generated:rolldown/runtime";
                    identifiers.set(id, reference);
                    description.inputs[reference] = {
                        kind: "generated",
                        path: id,
                        imports: [],
                    };
                }
                const reference = identifiers.get(id);
                if (!reference) {
                    throw new BuildError("BUILD_FAILED", `Unknown emitted module: ${id}`);
                }
                inputs.push(reference);
            }
            for (const [dynamic, paths] of [
                [false, file.imports],
                [true, file.dynamicImports],
            ] as const) {
                for (const imported of paths) {
                    const resolved = posix.normalize(
                        posix.join(posix.dirname(file.fileName), imported),
                    );
                    const internal = Object.hasOwn(bundle, imported)
                        ? imported
                        : Object.hasOwn(bundle, resolved)
                          ? resolved
                          : undefined;
                    imports.push({
                        path: internal ? `${output}/${internal}` : imported,
                        external: internal === undefined,
                        dynamic,
                    });
                }
            }
        }
        description.outputs[path] = {
            inputs: inputs.sort(),
            imports,
            exports: file.type === "chunk" ? file.exports : [],
        };
    }

    // stabilize package order after parallel module parsing
    description.packages = Object.fromEntries(
        Object.entries(description.packages).sort(([left], [right]) =>
            left < right ? -1 : left > right ? 1 : 0,
        ),
    );

    return description;
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
