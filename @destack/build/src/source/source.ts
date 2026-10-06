import { mkdtemp, realpath, rm, stat, writeFile, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve, isAbsolute } from "node:path";
import { type InspectOptions } from "../inspect/inspection.ts";
import { runtimeConditions, TYPE_CHECKS, TYPESCRIPT_OPTIONS } from "@destack/package/build";
import { linkDependencies } from "./dependency.ts";
import { PackageError } from "@destack/package/error";
import {
    PackageDefinition,
    PackageDescription,
    type PackageExport,
    TEST_EXPORT,
} from "@destack/package";
import { type JsonValue, schema } from "@destack/schema";
import { PackagePath } from "@destack/package/file";
import { type Runtime } from "@destack/package/runtime";
import type { ModulePackage } from "@destack/package/transform";
import { isMissing } from "../error/index.ts";

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
    /** The modules the configuration roots: exports, type declarations and extra modules. */
    entries: readonly string[];
}

/** Name the package a source compiles as the build releases it, with the metadata its modules receive. */
export function compiledPackage(
    source: Pick<PackageSource, "directory" | "declaration">,
): ModulePackage {
    return { directory: source.directory, metadata: { package: source.declaration.package } };
}

/** Open package inputs and prepare one compiler configuration for inspection and compilation. */
export async function openSource(
    options: InspectOptions & {
        /** Public modules selected for this output. */
        entries?: Readonly<Record<string, string>>;
        /** Framework modules compiled in addition to package exports. */
        files?: readonly string[];
        /** The version the build releases the package at, absent for the one its package.json names. */
        version?: string;
    },
): Promise<PackageSource> {
    // read the package declaration
    const definition = await readPackageDeclaration(
        options.directory,
        options.runtime,
        options.entries,
        options.version,
    );
    const authored = resolve(definition.directory, options.configuration ?? "tsconfig.json");

    // use an explicit configuration or the package's existing configuration
    let hasConfiguration = true;
    try {
        await stat(authored);
    } catch (error) {
        if (options.configuration !== undefined || !isMissing(error)) {
            throw error;
        }
        hasConfiguration = false;
    }

    // root the configuration at the selected exports, type declarations and extra modules
    const temporary = await mkdtemp(join(tmpdir(), "destack-project-"));
    const configuration = join(temporary, "tsconfig.json");
    const extra = (options.files ?? []).map((path) =>
        resolve(definition.directory, PackagePath.parse(path)),
    );
    const roots = [
        ...Object.values(definition.exports),
        ...Object.values(definition.types),
        ...extra,
    ];
    const files = [...new Set(roots.filter((path) => /\.[cm]?[jt]sx?$/u.test(path)))];
    try {
        // resolve explicit type packages from the same installation as authored modules
        await linkDependencies(definition.directory, temporary);
        const extended = hasConfiguration ? await realpath(authored) : undefined;
        const settings = compilerSettings(definition, files, extended);
        await writeFile(configuration, JSON.stringify(settings), { flag: "wx" });
    } catch (error) {
        await rm(temporary, { recursive: true });
        throw error;
    }

    return {
        ...definition,
        configuration,
        entries: files,
        async [Symbol.asyncDispose]() {
            await rm(temporary, { recursive: true });
        },
    };
}

/** Apply runtime resolution and the standard type checks to an authored configuration, or to generated defaults without one. */
function compilerSettings(
    definition: Pick<PackageSource, "directory" | "runtime">,
    files: readonly string[],
    authored: string | undefined,
): object {
    const conditions = runtimeConditions(definition.runtime);

    // extend the authored configuration with the standard type checks
    if (authored !== undefined) {
        return {
            extends: authored,
            compilerOptions: { ...TYPE_CHECKS, customConditions: conditions },
            files,
        };
    }
    // check the package's sources and tests with the default options
    else {
        return {
            compilerOptions: {
                ...TYPESCRIPT_OPTIONS,
                jsx: "preserve",
                jsxImportSource: "@destack/view",
                customConditions: conditions,
            },
            files,
            include: [
                resolve(definition.directory, "src/**/*.ts"),
                resolve(definition.directory, "src/**/*.tsx"),
                resolve(definition.directory, "test/**/*.test.ts"),
                resolve(definition.directory, "tests/**/*.test.ts"),
            ],
        };
    }
}

/** Read package manifests at the version the build releases, and select exports for one runtime. */
async function readPackageDeclaration(
    directory: string,
    runtime: Runtime,
    entries: Readonly<Record<string, string>> | undefined,
    version: string | undefined,
): Promise<Omit<PackageSource, "configuration" | "entries" | typeof Symbol.asyncDispose>> {
    // read the package's manifests
    directory = await realpath(directory);
    const declaration = await readPackageDescription(directory, version);
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
        // compile concrete exports the runtime supports
        if (!isCompiled(name, definition, runtime, entries !== undefined)) {
            continue;
        }

        // select the export's type declaration and runtime module
        const type = selectType(entry, typeConditions);
        if (type !== undefined) {
            types[name] = resolve(directory, type);
        }
        const module = selectModule(entry, conditions);
        if (module !== undefined) {
            exports[name] = resolve(directory, module);
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

/** Decide whether an output compiles an export, refusing invalid names and selected entries the runtime lacks. */
function isCompiled(
    name: string,
    definition: PackageDefinition,
    runtime: Runtime,
    isSelected: boolean,
): boolean {
    // require a subpath export naming a concrete module
    if (!isSelected && name !== "." && !name.startsWith("./")) {
        throw new PackageError("INVALID_DEFINITION", `invalid export: ${name}`);
    }
    if (name.includes("*")) {
        throw new PackageError(
            "INVALID_DEFINITION",
            `build exports must name concrete modules: ${name}`,
        );
    }

    // skip the test layer
    if (name === TEST_EXPORT && !isSelected) {
        return false;
    }

    // skip exports of other runtimes, refusing a selected entry of another runtime
    const isSupported = PackageDefinition.runtimes(definition, name).includes(runtime);
    if (!isSupported && isSelected) {
        throw new PackageError(
            "UNSUPPORTED_TARGET",
            `unsupported entry runtime: ${name} -> ${runtime}`,
        );
    }

    return isSupported;
}

/** Select an export's type declaration module by package path, absent without a script one. */
function selectType(entry: PackageExport, conditions: ReadonlySet<string>): string | undefined {
    // select a script declaration with a relative path
    const type = selectExport(entry, conditions);
    if (type === undefined || type === null || !/\.[cm]?[jt]sx?$/u.test(type)) {
        return undefined;
    }
    if (!type.startsWith("./")) {
        throw new PackageError("INVALID_DEFINITION", `invalid type export path: ${type}`);
    }

    return PackagePath.parse(type.slice(2));
}

/** Select an export's runtime module by package path, absent for no target or a declaration file. */
function selectModule(entry: PackageExport, conditions: ReadonlySet<string>): string | undefined {
    // select a relative target, leaving declaration files to the types
    const path = selectExport(entry, conditions);
    if (path === undefined || path === null) {
        return undefined;
    }
    if (!path.startsWith("./") || isAbsolute(path)) {
        throw new PackageError("INVALID_DEFINITION", `invalid export path: ${path}`);
    }

    return /\.d\.[cm]?ts$/u.test(path) ? undefined : PackagePath.parse(path.slice(2));
}

/** Read a package's manifests into its description, at the version a build releases it at when given. */
export async function readPackageDescription(
    directory: string,
    version?: string,
): Promise<PackageDescription> {
    // read package.json and destack.json
    const metadata = schema
        .record(schema.string(), schema.json())
        .parse(JSON.parse(await readFile(resolve(directory, "package.json"), "utf8")));
    const configuration = PackageDefinition.read(
        await readFile(resolve(directory, "destack.json"), "utf8"),
    );
    const declaration = PackageDescription.parse({
        package: {
            id: configuration.id,
            name: metadata["name"],
            version: version ?? metadata["version"],
        },
        definition: configuration,
        ...(metadata["exports"] === undefined ? {} : { exports: metadata["exports"] }),
        dependencies: field(metadata, "dependencies"),
        peerDependencies: field(metadata, "peerDependencies"),
        peerDependenciesMeta: field(metadata, "peerDependenciesMeta"),
        optionalDependencies: field(metadata, "optionalDependencies"),
        devDependencies: field(metadata, "devDependencies"),
    });
    const definition = declaration.definition;

    // require a TypeScript package
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

/** Read a package.json field, an empty object when absent. */
function field(metadata: Readonly<Record<string, JsonValue>>, name: string): JsonValue {
    return metadata[name] === undefined ? {} : metadata[name];
}

/** Key a package's exports by export name, such as `.` and `./server`. */
export function mapExports(
    declaration: PackageDescription,
): Readonly<Record<string, PackageExport>> {
    // require exports
    const authored = declaration.exports;
    if (authored === undefined) {
        throw new PackageError(
            "INVALID_DEFINITION",
            "a package build requires package.json exports",
        );
    }

    // key a conditions map by its subpaths, and a single target as the root export
    if (
        typeof authored === "object" &&
        authored !== null &&
        !Array.isArray(authored) &&
        Object.keys(authored).some((name) => name.startsWith("."))
    ) {
        return authored;
    }

    return { ".": authored };
}

/** Select a package export using declaration-order conditional resolution. */
export function selectExport(
    value: PackageExport | undefined,
    conditions: ReadonlySet<string>,
): string | null | undefined {
    // return a target as it is
    if (typeof value === "string" || value === null) {
        return value;
    }
    // refuse an absent definition
    else if (value === undefined) {
        throw new PackageError("INVALID_DEFINITION", "invalid package export definition");
    }

    // try a fallback list's alternatives, or the matching conditions in declaration order
    const candidates = Array.isArray(value)
        ? value
        : Object.entries(value).flatMap(([condition, entry]) =>
              conditions.has(condition) ? [entry] : [],
          );
    for (const candidate of candidates) {
        const selected = selectExport(candidate, conditions);
        if (selected !== undefined) {
            return selected;
        }
    }

    return undefined;
}
