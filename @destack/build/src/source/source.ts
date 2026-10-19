import { mkdtemp, realpath, rm, stat, writeFile, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve, isAbsolute } from "node:path";
import { type InspectOptions } from "../inspection/inspection.ts";
import { runtimeConditions, TYPE_CHECKS, TYPESCRIPT_OPTIONS } from "@destack/package/build";
import { linkDependencies, ReleasedPackage } from "./dependency.ts";
import { PackageError } from "@destack/package/error";
import { PackageDefinition, PackageJson, type PackageExport, TEST_EXPORT } from "@destack/package";
import { type JsonObject, type JsonValue, schema, type Version } from "@destack/schema";
import { PackagePath } from "@destack/package/file";
import { type Runtime } from "@destack/package/runtime";
import { PackageLocator } from "@destack/package/locator";
import { isMissing } from "../error/index.ts";

/** Source manifests and selected runtime exports for a package build. */
export interface PackageSource extends AsyncDisposable {
    /** The absolute compiler configuration path. */
    configuration: string;
    /** The absolute package directory. */
    directory: string;
    /** The package.json and destack.json, at the versions the build releases. */
    packageJson: PackageJson;
    /** The versions the build releases its source packages at, by package name. */
    versions: Readonly<Record<string, Version>>;
    /** Source files keyed by public export name. */
    exports: Record<string, string>;
    /** Source declarations keyed by public export name. */
    types: Record<string, string>;
    /** The runtime used for conditional dependency resolution and compilation. */
    runtime: Runtime;
    /** The modules the configuration roots: exports, type declarations and extra modules. */
    entries: readonly string[];
}

/** A package's inputs opened for inspection and compilation. */
export const PackageSource = {
    /** Open package inputs and prepare one compiler configuration. */
    async open(
        options: InspectOptions & {
            /** Public modules selected for this output. */
            entries?: Readonly<Record<string, string>>;
            /** Framework modules compiled in addition to package exports. */
            files?: readonly string[];
            /** The versions the build releases its source packages at, by package name. */
            versions?: Readonly<Record<string, Version>>;
        },
    ): Promise<PackageSource> {
        // read the package.json and destack.json and select the runtime's exports
        const definition = await readPackageSource(
            options.directory,
            options.runtime,
            options.entries,
            options.versions ?? {},
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
    },
};

/** Apply runtime resolution and the standard type checks to a configuration or defaults. */
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

/** Read a package at the versions a build releases, selecting its exports for one runtime. */
async function readPackageSource(
    directory: string,
    runtime: Runtime,
    entries: Readonly<Record<string, string>> | undefined,
    versions: Readonly<Record<string, Version>>,
): Promise<Omit<PackageSource, "configuration" | "entries" | typeof Symbol.asyncDispose>> {
    // read the package.json and destack.json
    directory = await realpath(directory);
    const packageJson = await readPackageJson(directory, versions);
    const definition = packageJson.definition;

    // select package exports using the runtime's standard conditions
    const conditions = new Set(["import", "default", ...runtimeConditions(runtime)]);
    const typeConditions = new Set(["types", ...conditions]);
    const authoredEntries = entries
        ? Object.fromEntries(
              Object.entries(entries).map(([name, path]) => [name, `./${PackagePath.parse(path)}`]),
          )
        : mapExports(packageJson);
    const exports: Record<string, string> = {};
    const types: Record<string, string> = {};
    for (const [name, entry] of Object.entries(authoredEntries)) {
        // compile concrete exports the runtime supports
        if (!isCompiled(name, definition, runtime, entries !== undefined)) {
            continue;
        }

        // select the export's type packageJson and runtime module
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
        packageJson,
        versions,
        exports,
        types,
        runtime,
    };
}

/** Decide whether an output compiles an export, refusing invalid names and missing entries. */
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

/** Select an export's runtime module by path, absent for no target or a declaration file. */
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

/** Read a package's package.json and destack.json, at the version a build releases it at. */
export async function readPackageJson(
    directory: string,
    versions: Readonly<Record<string, Version>> = {},
): Promise<PackageJson> {
    // read package.json and destack.json
    const metadata = schema
        .jsonObject()
        .parse(JSON.parse(await readFile(resolve(directory, "package.json"), "utf8")));
    const configuration = PackageDefinition.read(
        await readFile(resolve(directory, "destack.json"), "utf8"),
    );
    const packageJson = PackageJson.parse({
        package: {
            id: configuration.id,
            name: metadata["name"],
            version: PackageLocator.release(directory, ReleasedPackage.parse(metadata), versions),
        },
        definition: configuration,
        ...(metadata["exports"] === undefined ? {} : { exports: metadata["exports"] }),
        dependencies: field(metadata, "dependencies"),
        peerDependencies: field(metadata, "peerDependencies"),
        peerDependenciesMeta: field(metadata, "peerDependenciesMeta"),
        optionalDependencies: field(metadata, "optionalDependencies"),
        devDependencies: field(metadata, "devDependencies"),
    });
    const definition = packageJson.definition;

    // require each runtime override to name an export of package.json
    const overridden = Object.keys(definition.exports ?? {});
    const exported = overridden.length === 0 ? {} : mapExports(packageJson);
    const stale = overridden.find((name) => !Object.hasOwn(exported, name));
    if (stale !== undefined) {
        throw new PackageError(
            "INVALID_DEFINITION",
            `destack.json overrides the runtimes of an export package.json lacks: ${stale}`,
        );
    }

    // require a TypeScript package
    if (definition.language !== "typescript") {
        throw new PackageError(
            "UNSUPPORTED_LANGUAGE",
            "building TypeScript++ packages requires the language toolchain",
        );
    }

    // associate peer metadata with authored dependency requirements
    for (const name of Object.keys(packageJson.peerDependenciesMeta)) {
        if (!Object.hasOwn(packageJson.peerDependencies, name)) {
            throw new PackageError("INVALID_DEPENDENCY", `undeclared peer dependency: ${name}`);
        }
    }

    return packageJson;
}

/** Read a package.json field, an empty object when absent. */
function field(metadata: JsonObject, name: string): JsonValue {
    return metadata[name] ?? {};
}

/** Key a package's exports by export name, such as `.` and `./server`. */
export function mapExports(packageJson: PackageJson): Readonly<Record<string, PackageExport>> {
    // require exports
    const authored = packageJson.exports;
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
