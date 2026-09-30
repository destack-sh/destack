import { mkdtemp, realpath, rm, stat, writeFile, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve, isAbsolute } from "node:path";
import { type InspectOptions } from "../inspect/inspection.ts";
import { runtimeConditions } from "@destack/package/build";
import { linkDependencies } from "./dependency.ts";
import { PackageError } from "@destack/package/error";
import { PackageDefinition, PackageDescription, Publication } from "@destack/package";
import { schema } from "@destack/schema";
import { PackagePath } from "@destack/package/file";
import { type Runtime } from "@destack/package/runtime";

/** Source manifests and selected runtime exports for a package build. */
export interface PackageSource extends AsyncDisposable {
    /** The absolute compiler configuration path. */
    configuration: string;
    /** The absolute package directory. */
    directory: string;
    /** Combined authored package declarations. */
    declaration: PackageDescription;
    /** Source files keyed by public export name. */
    exports: Record<string, string>;
    /** Source declarations keyed by public export name. */
    types: Record<string, string>;
    /** The runtime used for conditional dependency resolution and compilation. */
    runtime: Runtime;
}

/** Open package inputs and prepare one compiler configuration for inspection and compilation. */
export async function openSource(
    options: InspectOptions & {
        /** Public modules selected for this output. */
        entries?: Readonly<Record<string, string>>;
        /** Framework modules compiled in addition to package exports. */
        files?: readonly string[];
    },
): Promise<PackageSource> {
    // read the package declaration
    const definition = await readPackageDeclaration(
        options.directory,
        options.runtime,
        options.entries,
    );
    const authored = resolve(definition.directory, options.configuration ?? "tsconfig.json");

    // use an explicit configuration or the package's existing configuration
    let hasConfiguration = true;
    try {
        await stat(authored);
    } catch (error) {
        if (
            options.configuration !== undefined ||
            !(error instanceof Error && "code" in error && error.code === "ENOENT")
        ) {
            throw error;
        }
        hasConfiguration = false;
    }

    // apply runtime resolution to authored configurations and generated defaults alike
    const temporary = await mkdtemp(join(tmpdir(), "destack-project-"));
    const configuration = join(temporary, "tsconfig.json");
    try {
        // resolve explicit type packages from the same installation as authored modules
        await linkDependencies(definition.directory, temporary);
        const files = [
            ...Object.values(definition.exports),
            ...Object.values(definition.types),
            ...(options.files ?? []).map((path) =>
                resolve(definition.directory, PackagePath.parse(path)),
            ),
        ].filter((path) => /\.[cm]?[jt]sx?$/.test(path));
        const conditions = runtimeConditions(definition.runtime);
        const settings = hasConfiguration
            ? {
                  extends: await realpath(authored),
                  compilerOptions: { customConditions: conditions },
                  files: [...new Set(files)],
              }
            : {
                  compilerOptions: {
                      target: "ESNext",
                      module: "ESNext",
                      moduleResolution: "bundler",
                      allowImportingTsExtensions: true,
                      noEmit: true,
                      strict: true,
                      skipLibCheck: true,
                      jsx: "preserve",
                      jsxImportSource: "@destack/view",
                      customConditions: conditions,
                  },
                  files: [...new Set(files)],
                  include: [
                      resolve(definition.directory, "src/**/*.ts"),
                      resolve(definition.directory, "src/**/*.tsx"),
                      resolve(definition.directory, "test/**/*.test.ts"),
                      resolve(definition.directory, "tests/**/*.test.ts"),
                  ],
              };

        await writeFile(configuration, JSON.stringify(settings), { flag: "wx" });
    } catch (error) {
        await rm(temporary, { recursive: true });
        throw error;
    }

    return {
        ...definition,
        configuration,
        async [Symbol.asyncDispose]() {
            await rm(temporary, { recursive: true });
        },
    };
}

/** Read package manifests and select exports for one runtime. */
async function readPackageDeclaration(
    directory: string,
    runtime: Runtime,
    entries?: Readonly<Record<string, string>>,
): Promise<Omit<PackageSource, "configuration" | typeof Symbol.asyncDispose>> {
    // read the package's manifests
    directory = await realpath(directory);
    const declaration = await readPackageDescription(directory);
    const definition = declaration.definition;

    // select package exports using the runtime's standard conditions
    const conditions = new Set(["import", "default", ...runtimeConditions(runtime)]);
    const typeConditions = new Set(["types", ...conditions]);
    const authoredEntries = entries
        ? Object.fromEntries(
              Object.entries(entries).map(([name, path]) => [name, `./${PackagePath.parse(path)}`]),
          )
        : mapExports(declaration);
    const exports: Record<string, string> = {};
    const types: Record<string, string> = {};
    for (const [name, entry] of Object.entries(authoredEntries)) {
        if (!entries && name !== "." && !name.startsWith("./")) {
            throw new PackageError("INVALID_DEFINITION", `invalid export: ${name}`);
        }
        if (name.includes("*")) {
            throw new PackageError(
                "INVALID_DEFINITION",
                `build exports must name concrete modules: ${name}`,
            );
        }
        if (!PackageDefinition.runtimes(definition, name).includes(runtime)) {
            if (entries) {
                throw new PackageError(
                    "UNSUPPORTED_TARGET",
                    `unsupported entry runtime: ${name} -> ${runtime}`,
                );
            }
            continue;
        }
        const type = selectExport(entry, typeConditions);
        if (type !== undefined && type !== null && /\.[cm]?[jt]sx?$/.test(type)) {
            if (!type.startsWith("./")) {
                throw new PackageError("INVALID_DEFINITION", `invalid type export path: ${type}`);
            }
            types[name] = resolve(directory, PackagePath.parse(type.slice(2)));
        }
        const path = selectExport(entry, conditions);
        if (path === undefined || path === null) {
            continue;
        }
        if (!path.startsWith("./") || isAbsolute(path)) {
            throw new PackageError("INVALID_DEFINITION", `invalid export path: ${path}`);
        }
        if (!/\.d\.[cm]?ts$/.test(path)) {
            exports[name] = resolve(directory, PackagePath.parse(path.slice(2)));
        }
    }

    // require at least one runtime or type export
    if (!Object.keys(exports).length && !Object.keys(types).length) {
        throw new PackageError("UNSUPPORTED_TARGET", `no exports for runtime: ${runtime}`);
    }

    return {
        directory,
        declaration,
        exports,
        types,
        runtime,
    };
}

/** Read a package's manifests into its description. */
export async function readPackageDescription(directory: string): Promise<PackageDescription> {
    // read package.json and destack.json
    const metadata = schema
        .record(schema.string(), schema.json())
        .parse(JSON.parse(await readFile(resolve(directory, "package.json"), "utf8")));
    const configuration = PackageDefinition.read(
        await readFile(resolve(directory, "destack.json"), "utf8"),
    );
    const declaration = PackageDescription.parse({
        package: { id: configuration.id, name: metadata.name, version: metadata.version },
        definition: configuration,
        exports: metadata.exports,
        dependencies: metadata.dependencies === undefined ? {} : metadata.dependencies,
        peerDependencies: metadata.peerDependencies === undefined ? {} : metadata.peerDependencies,
        peerDependenciesMeta:
            metadata.peerDependenciesMeta === undefined ? {} : metadata.peerDependenciesMeta,
        optionalDependencies:
            metadata.optionalDependencies === undefined ? {} : metadata.optionalDependencies,
        devDependencies: metadata.devDependencies === undefined ? {} : metadata.devDependencies,
    });
    const definition = declaration.definition;

    // require a publishable TypeScript package
    if (definition.publication !== undefined) {
        Publication.require(definition.publication);
    }
    if (definition.language !== "typescript") {
        throw new PackageError(
            "UNSUPPORTED_LANGUAGE",
            "building TypeScript++ packages requires the language toolchain",
        );
    }

    // associate peer metadata with authored dependency requirements
    for (const name of Object.keys(declaration.peerDependenciesMeta)) {
        if (!Object.hasOwn(declaration.peerDependencies, name)) {
            throw new PackageError("INVALID_DEPENDENCY", `undeclared peer dependency: ${name}`);
        }
    }

    return declaration;
}

/** Key a package's exports by export name, such as `.` and `./server`. */
export function mapExports(declaration: PackageDescription): Readonly<Record<string, unknown>> {
    // require exports
    const authored = declaration.exports;
    if (authored === undefined) {
        throw new PackageError(
            "INVALID_DEFINITION",
            "a package build requires package.json exports",
        );
    }
    const isMap =
        typeof authored === "object" &&
        authored !== null &&
        !Array.isArray(authored) &&
        Object.keys(authored).some((name) => name.startsWith("."));

    return isMap ? authored : { ".": authored };
}

/** Select a package export using declaration-order conditional resolution. */
export function selectExport(value: unknown, conditions: Set<string>): string | null | undefined {
    if (typeof value === "string" || value === null) {
        return value;
    }
    if (Array.isArray(value)) {
        for (const entry of value) {
            const selected = selectExport(entry, conditions);
            if (selected !== undefined) {
                return selected;
            }
        }
    } else if (typeof value === "object" && value !== null) {
        for (const [condition, entry] of Object.entries(value)) {
            if (conditions.has(condition)) {
                const selected = selectExport(entry, conditions);
                if (selected !== undefined) {
                    return selected;
                }
            }
        }
    } else {
        throw new PackageError("INVALID_DEFINITION", "invalid package export definition");
    }

    return undefined;
}
