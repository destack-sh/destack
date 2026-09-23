import { mkdtemp, realpath, rm, stat, writeFile, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve, isAbsolute } from "node:path";
import { type InspectOptions } from "../inspect/inspection.ts";
import { runtimeConditions } from "../compile/runtime.ts";
import { linkDependencies } from "./dependency.ts";
import { PackageError } from "@destack/package/error";
import { mergeCompute, PackageDeclaration, type Target } from "@destack/package";
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
    declaration: PackageDeclaration;
    /** Source files keyed by public export name. */
    exports: Record<string, string>;
    /** Source declarations keyed by public export name. */
    types: Record<string, string>;
    /** The runtime used for conditional dependency resolution and compilation. */
    runtime: Runtime;
}

/** Open package inputs and prepare one compiler configuration for inspection and compilation. */
export async function openPackage(
    options: InspectOptions & {
        /** Public modules selected for this output. */
        entries?: Readonly<Record<string, string>>;
        /** Framework modules compiled in addition to package exports. */
        files?: readonly string[];
    },
): Promise<PackageSource> {
    const definition = await readPackageDeclaration(
        options.directory,
        options.target,
        options.entries,
        options.runtime,
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

/** Read package manifests and select exports for one target. */
export async function readPackageDeclaration(
    directory: string,
    target: Target,
    entries?: Readonly<Record<string, string>>,
    runtime: Runtime = target === "browser" ? "browser" : "bun",
): Promise<Omit<PackageSource, "configuration" | typeof Symbol.asyncDispose>> {
    if ((target === "browser") !== (runtime === "browser")) {
        throw new PackageError(
            "UNSUPPORTED_TARGET",
            `Runtime ${runtime} cannot compile target ${target}.`,
        );
    }
    directory = await realpath(directory);
    const metadata = schema
        .record(schema.string(), schema.json())
        .parse(JSON.parse(await readFile(resolve(directory, "package.json"), "utf8")));
    const configuration = JSON.parse(await readFile(resolve(directory, "destack.json"), "utf8"));
    const declaration = PackageDeclaration.parse({
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
    if (definition.language !== "typescript") {
        throw new PackageError(
            "UNSUPPORTED_LANGUAGE",
            "TypeScript++ builds require the language toolchain.",
        );
    }

    // associate peer metadata with authored dependency requirements
    for (const name of Object.keys(declaration.peerDependenciesMeta)) {
        if (!Object.hasOwn(declaration.peerDependencies, name)) {
            throw new PackageError("INVALID_DEPENDENCY", `Undeclared peer dependency: ${name}`);
        }
    }

    // check workload selection against authored exports before applying target filters
    const declaredExports = declaration.exports;
    const names =
        declaredExports &&
        typeof declaredExports === "object" &&
        !Array.isArray(declaredExports) &&
        Object.keys(declaredExports).some((name) => name.startsWith("."))
            ? Object.keys(declaredExports)
            : declaredExports === undefined || declaredExports === null
              ? []
              : ["."];
    mergeCompute(definition.compute);

    // require named views to reference explicit browser exports
    for (const [name, view] of Object.entries(definition.views ?? {})) {
        const targets = definition.exports?.[view.entrypoint]?.targets ?? definition.targets;
        if (!names.includes(view.entrypoint) || !targets?.includes("browser")) {
            throw new PackageError(
                "INVALID_DEFINITION",
                `view ${name} requires a browser export: ${view.entrypoint}`,
            );
        }
    }

    for (const [name, workload] of Object.entries(definition.workloads ?? {})) {
        if (!names.includes(workload.entrypoint)) {
            throw new PackageError(
                "INVALID_DEFINITION",
                `Workload ${name} names an undeclared export: ${workload.entrypoint}`,
            );
        }
    }

    // select package exports using the target's standard conditions
    const conditions = new Set(["import", "default", ...runtimeConditions(runtime)]);
    const typeConditions = new Set(["types", ...conditions]);
    const authored = entries
        ? Object.fromEntries(
              Object.entries(entries).map(([name, path]) => [name, `./${PackagePath.parse(path)}`]),
          )
        : declaration.exports;
    if (authored === undefined) {
        throw new PackageError(
            "INVALID_DEFINITION",
            "A package build requires package.json exports.",
        );
    }
    const authoredEntries =
        typeof authored === "object" &&
        authored !== null &&
        !Array.isArray(authored) &&
        (entries || Object.keys(authored).some((name) => name.startsWith(".")))
            ? authored
            : { ".": authored };
    const exports: Record<string, string> = {};
    const types: Record<string, string> = {};
    for (const [name, entry] of Object.entries(authoredEntries)) {
        if (!entries && name !== "." && !name.startsWith("./")) {
            throw new PackageError("INVALID_DEFINITION", `Invalid export: ${name}`);
        }
        if (name.includes("*")) {
            throw new PackageError(
                "INVALID_DEFINITION",
                `Build exports must name concrete modules: ${name}`,
            );
        }
        const targets = definition.exports?.[name]?.targets ?? definition.targets;
        const runtimes = definition.exports?.[name]?.runtimes ?? definition.runtimes;
        if (runtimes && !runtimes.includes(runtime)) {
            if (entries) {
                throw new PackageError(
                    "UNSUPPORTED_TARGET",
                    `Unsupported entry runtime: ${name} -> ${runtime}`,
                );
            }
            continue;
        }
        if (!targets) {
            throw new PackageError("INVALID_DEFINITION", `No targets declared for export: ${name}`);
        }
        if (!targets.includes(target)) {
            if (entries) {
                throw new PackageError(
                    "UNSUPPORTED_TARGET",
                    `Unsupported entry target: ${name} -> ${target}`,
                );
            }
            continue;
        }
        const type = selectExport(entry, typeConditions);
        if (type !== undefined && type !== null && /\.[cm]?[jt]sx?$/.test(type)) {
            if (!type.startsWith("./")) {
                throw new PackageError("INVALID_DEFINITION", `Invalid type export path: ${type}`);
            }
            types[name] = resolve(directory, PackagePath.parse(type.slice(2)));
        }
        const path = selectExport(entry, conditions);
        if (path === undefined || path === null) {
            continue;
        }
        if (!path.startsWith("./") || isAbsolute(path)) {
            throw new PackageError("INVALID_DEFINITION", `Invalid export path: ${path}`);
        }
        if (!/\.d\.[cm]?ts$/.test(path)) {
            exports[name] = resolve(directory, PackagePath.parse(path.slice(2)));
        }
    }

    if (!Object.keys(exports).length && !Object.keys(types).length) {
        throw new PackageError("UNSUPPORTED_TARGET", `No runtime exports for target: ${target}`);
    }

    return {
        directory,
        declaration,
        exports,
        types,
        runtime,
    };
}

/** Select a package export using declaration-order conditional resolution. */
function selectExport(value: unknown, conditions: Set<string>): string | null | undefined {
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
        throw new PackageError("INVALID_DEFINITION", "Invalid package export definition.");
    }

    return undefined;
}
