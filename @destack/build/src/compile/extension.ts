import { HOST_RUNTIME } from "@destack/package/runtime";
import { DependencyTree } from "../dependency/tree.ts";
import { join } from "node:path";
import type { FileSystem } from "@destack/environment";
import {
    type BuildExtension,
    type PluginOption,
    runtimeConditions,
    type TransformContext,
} from "@destack/package/build";
import { PackageDefinition, PackageJson, PackageExport } from "@destack/package";
import { PackageLocator } from "@destack/package/locator";
import { BuildError, extensionFailureOf } from "../error/index.ts";
import { readPackageJson, selectExport } from "../source/index.ts";
import { present, schema } from "@destack/schema";

/** The exports of a package.json. */
const Manifest = schema.looseObject({
    exports: schema.record(schema.string(), PackageExport).exactOptional(),
});

/** The exports of a module. */
const Exports = schema.record(schema.string(), schema.unknown());

/** The conditions selecting an extension's module in the build worker. */
const CONDITIONS = new Set(["import", "default", ...runtimeConditions(HOST_RUNTIME)]);

/** A build extension with the directory of the package declaring it. */
export interface LoadedExtension {
    /** The declaring package's directory. */
    readonly directory: string;
    /** The extension. */
    readonly extension: BuildExtension;
}

/** Load the build extensions of a package, its listed extensions and its runtime closure. */
export async function loadExtensions(
    directory: string,
    packageJson: PackageJson,
    tree: DependencyTree,
    files: FileSystem,
): Promise<LoadedExtension[]> {
    // take the package itself, then its listed extensions and its runtime dependency closure
    const root = packageJson.package.name;
    const listed = packageJson.definition.extensions ?? [];
    const locator = new PackageLocator(files);
    const packages = new Map<string, { directory: string; definition: PackageDefinition }>([
        [root, { directory, definition: packageJson.definition }],
    ]);
    const required = PackageJson.runtimeDependencies(packageJson);
    const pending = [...new Set([...required, ...listed])].map((name) => ({
        name,
        dependent: packageJson,
        from: directory,
        isRequired: required.includes(name),
    }));
    for (const { name, dependent, from, isRequired } of pending) {
        const installed = packages.has(name)
            ? undefined
            : (tree.directories[name] ?? (await locator.directory(name, from)));
        // take each package once, and skip an absent optional dependency
        if (
            packages.has(name) ||
            (installed === undefined && PackageJson.isOptional(dependent, name))
        ) {
            continue;
        }
        // refuse another absent one
        else if (installed === undefined) {
            const named = dependent.package.name;
            throw BuildError.of("DEPENDENCY_UNRESOLVED", `missing dependency of ${named}: ${name}`);
        }
        // take a Destack package's definition, walking a required one's runtime dependencies
        else if (await files.exists(join(installed, "destack.json"))) {
            const read = await readPackageJson(installed, files);
            packages.set(name, { directory: installed, definition: read.definition });
            const next = isRequired ? PackageJson.runtimeDependencies(read) : [];
            pending.push(
                ...next.map((each) => ({
                    name: each,
                    dependent: read,
                    from: installed,
                    isRequired: true,
                })),
            );
        }
    }

    // refuse a listed extension that is not one
    const unbuilt = listed.find((name) => packages.get(name)?.definition.build === undefined);
    if (unbuilt !== undefined) {
        throw BuildError.of(
            "EXTENSION_INVALID",
            `${root} lists ${unbuilt}, but ${unbuilt} declares no build extension`,
        );
    }

    // import each declared extension in name order
    const extensions: LoadedExtension[] = [];
    for (const name of [...packages.keys()].toSorted()) {
        const found = present(packages.get(name), `the package ${name}`);
        if (found.definition.build === undefined) {
            continue;
        }
        const extension = await importExtension(
            name,
            found.directory,
            found.definition.build,
            files,
        );
        extensions.push({ directory: found.directory, extension });
    }

    return extensions;
}

/** Load the dependency closure extensions' module transforms for development. */
export async function loadTransforms(
    context: Omit<TransformContext, "dependencies">,
): Promise<PluginOption[]> {
    // develop from the sources, where no dependency's build is materialized
    const { files } = context.environment;
    const packageJson = await readPackageJson(context.directory, files);
    const tree = await DependencyTree.read({}, files);
    const loaded = await loadExtensions(context.directory, packageJson, tree, files);

    return transformPlugins(
        loaded.map((entry) => entry.extension),
        { ...context, dependencies: tree.directories },
    );
}

/** Collect the extensions' module transform plugins, reporting failures as build failures. */
export function transformPlugins(
    extensions: readonly BuildExtension[],
    context: TransformContext,
): PluginOption[] {
    return extensions.flatMap((extension) => {
        try {
            return extension.transform?.(context) ?? [];
        } catch (cause) {
            throw extensionFailureOf(cause);
        }
    });
}

/** Read the exports of an installed package's package.json. */
async function readManifest(
    installed: string,
    files: FileSystem,
): Promise<schema.Infer<typeof Manifest>> {
    return Manifest.parse(JSON.parse(await files.readTextFile(join(installed, "package.json"))));
}

/** Import the extension a package's build field names, such as `./build#viewExtension`. */
async function importExtension(
    name: string,
    installed: string,
    build: string,
    files: FileSystem,
): Promise<BuildExtension> {
    // split the build field into the package export and the export name
    const separator = build.indexOf("#");
    if (separator === -1) {
        throw BuildError.of(
            "EXTENSION_INVALID",
            `${name} declares build ${build}, but has no export ${build}`,
        );
    }
    const subpath = build.slice(0, separator);
    const exportName = build.slice(separator + 1);

    // select the module the build worker's conditions resolve
    const manifest = await readManifest(installed, files);
    const file = selectExport(manifest.exports?.[subpath], CONDITIONS);
    if (typeof file !== "string") {
        throw BuildError.of("EXTENSION_INVALID", `${name} exports no build module at ${subpath}`);
    }

    // read the extension from the module's exports
    const module = Exports.parse(await import(join(installed, file)));
    const extension = module[exportName];
    if (typeof extension !== "object" || extension === null) {
        throw BuildError.of(
            "EXTENSION_INVALID",
            `${name} exports no build extension ${exportName}`,
        );
    }

    return extension;
}
