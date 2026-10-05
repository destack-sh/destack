import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import {
    type BuildExtension,
    type PluginOption,
    runtimeConditions,
    type TransformContext,
} from "@destack/package/build";
import { PackageDefinition, type PackageDescription, PackageExport } from "@destack/package";
import { registerModulePlugin } from "@destack/package/bun";
import { PackageLocator } from "@destack/package/transform";
import { BuildError } from "../error/index.ts";
import { readPackageDescription, selectExport } from "../source/index.ts";
import { present, schema } from "@destack/schema";

/** The exports and requirements of a package.json. */
const Manifest = schema.looseObject({
    exports: schema.record(schema.string(), PackageExport).exactOptional(),
    dependencies: schema.record(schema.string(), schema.string()).default({}),
    peerDependencies: schema.record(schema.string(), schema.string()).default({}),
    peerDependenciesMeta: schema
        .record(schema.string(), schema.looseObject({ optional: schema.boolean().exactOptional() }))
        .default({}),
    optionalDependencies: schema.record(schema.string(), schema.string()).default({}),
});

/** The exports of a module. */
const Exports = schema.record(schema.string(), schema.unknown());

/** The conditions selecting an extension's module in the build worker. */
const CONDITIONS = new Set(["import", "default", ...runtimeConditions("bun")]);

/** The requirements of a package that decide which dependencies it builds without. */
interface Requirements {
    /** The required dependencies. */
    readonly dependencies: Readonly<Record<string, string>>;
    /** The dependencies a dependent supplies. */
    readonly peerDependencies: Readonly<Record<string, string>>;
    /** Whether each peer is optional. */
    readonly peerDependenciesMeta: Readonly<Record<string, { readonly optional?: boolean }>>;
    /** The optional dependencies. */
    readonly optionalDependencies: Readonly<Record<string, string>>;
}

/** A build extension with the directory of the package declaring it. */
export interface LoadedExtension {
    /** The declaring package's directory. */
    readonly directory: string;
    /** The extension. */
    readonly extension: BuildExtension;
}

/** Load each extension of the package's dependency closure once in name order, its development dependencies included in development. */
export async function loadExtensions(
    directory: string,
    declaration: PackageDescription,
    options: { readonly development?: boolean } = {},
): Promise<LoadedExtension[]> {
    // walk the Destack packages the package requires, directly or through each other
    const own = declaration.package.name;
    const locator = new PackageLocator();
    const packages = new Map<string, { directory: string; definition: PackageDefinition }>();
    const requirements: Requirements =
        options.development === true
            ? {
                  ...declaration,
                  dependencies: { ...declaration.devDependencies, ...declaration.dependencies },
              }
            : declaration;
    const pending: { name: string; directory: string; requirements: Requirements }[] = [
        { name: own, directory, requirements },
    ];
    packages.set(own, { directory, definition: declaration.definition });
    for (let entry = pending.pop(); entry !== undefined; entry = pending.pop()) {
        for (const name of requiredNames(entry.requirements)) {
            // visit each package once
            if (packages.has(name)) {
                continue;
            }

            // skip an absent optional dependency and refuse an absent required one
            const installed = locator.directory(name, entry.directory);
            if (installed === undefined) {
                if (isOptional(name, entry.requirements)) {
                    continue;
                }
                throw new BuildError(
                    "BUILD_FAILED",
                    `missing dependency of ${entry.name}: ${name}`,
                );
            }

            // walk on through Destack packages alone
            const path = join(installed, "destack.json");
            if (!existsSync(path)) {
                continue;
            }
            const definition = PackageDefinition.read(await readFile(path, "utf8"));
            packages.set(name, { directory: installed, definition });
            const manifest = await readManifest(installed);
            pending.push({ name, directory: installed, requirements: manifest });
        }
    }

    // import each declared extension in name order
    const extensions: LoadedExtension[] = [];
    for (const name of [...packages.keys()].toSorted()) {
        const found = present(packages.get(name), `the package ${name}`);
        if (found.definition.build === undefined) {
            continue;
        }
        const extension = await importExtension(name, found.directory, found.definition.build);
        extensions.push({ directory: found.directory, extension });
    }

    return extensions;
}

/** Load the plugins the extensions of a package's dependency closure, its development dependencies included, transform its modules with in development. */
export async function loadTransforms(context: TransformContext): Promise<PluginOption[]> {
    const declaration = await readPackageDescription(context.directory);
    const loaded = await loadExtensions(context.directory, declaration, { development: true });

    return transformPlugins(
        loaded.map((entry) => entry.extension),
        context,
    );
}

/** Collect the plugins extensions transform a package's modules with, reporting a failure as a build failure. */
export function transformPlugins(
    extensions: readonly BuildExtension[],
    context: TransformContext,
): PluginOption[] {
    return extensions.flatMap((extension) => {
        try {
            return extension.transform?.(context) ?? [];
        } catch (cause) {
            throw BuildError.from(cause);
        }
    });
}

/** List the names a package requires at run time, its own dependencies, peers and optional ones. */
function requiredNames(requirements: Requirements): string[] {
    return Object.keys({
        ...requirements.dependencies,
        ...requirements.peerDependencies,
        ...requirements.optionalDependencies,
    });
}

/** Read the exports and requirements of an installed package's package.json. */
async function readManifest(installed: string): Promise<schema.Infer<typeof Manifest>> {
    return Manifest.parse(JSON.parse(await readFile(join(installed, "package.json"), "utf8")));
}

/** Import the extension a package's build field names, such as `./build#viewExtension`. */
async function importExtension(
    name: string,
    installed: string,
    build: string,
): Promise<BuildExtension> {
    // split the build field into the package export and the export name
    const separator = build.indexOf("#");
    if (separator === -1) {
        throw new BuildError(
            "BUILD_FAILED",
            `${name} declares the build ${build}, which names no export`,
        );
    }
    const subpath = build.slice(0, separator);
    const exportName = build.slice(separator + 1);

    // select the module the build worker's conditions resolve
    const manifest = await readManifest(installed);
    const file = selectExport(manifest.exports?.[subpath], CONDITIONS);
    if (typeof file !== "string") {
        throw new BuildError("BUILD_FAILED", `${name} exports no build module at ${subpath}`);
    }

    // read the extension from the module's exports, loading package sources with their module metadata
    registerModulePlugin();
    const module = Exports.parse(await import(join(installed, file)));
    const extension = module[exportName];
    if (typeof extension !== "object" || extension === null) {
        throw new BuildError("BUILD_FAILED", `${name} exports no build extension ${exportName}`);
    }

    return extension;
}

/** Decide whether a package builds without a dependency, as an optional one or an optional peer. */
function isOptional(name: string, requirements: Requirements): boolean {
    // accept an optional dependency, which overrides a matching dependency
    if (Object.hasOwn(requirements.optionalDependencies, name)) {
        return true;
    }

    // accept an optional peer the package does not also require
    const isRequired = Object.hasOwn(requirements.dependencies, name);
    const isOptionalPeer = requirements.peerDependenciesMeta[name]?.optional === true;

    return !isRequired && isOptionalPeer;
}
