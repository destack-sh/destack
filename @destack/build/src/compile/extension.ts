import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { type BuildExtension, runtimeConditions } from "@destack/package/build";
import { PackageDefinition, type PackageDescription, PackageExport } from "@destack/package";
import { PackageLocator } from "@destack/package/transform";
import { BuildError } from "../error/index.ts";
import { selectExport } from "../source/index.ts";
import { schema } from "@destack/schema";

/** The exports of a package.json. */
const Manifest = schema.looseObject({
    exports: schema.record(schema.string(), PackageExport).exactOptional(),
});

/** The exports of a module. */
const Exports = schema.record(schema.string(), schema.unknown());

/** The conditions selecting an extension's module in the build worker. */
const CONDITIONS = new Set(["import", "default", ...runtimeConditions("bun")]);

/** A build extension with the directory of the package declaring it. */
export interface LoadedExtension {
    /** The declaring package's directory. */
    readonly directory: string;
    /** The extension. */
    readonly extension: BuildExtension;
}

/** Load the extensions the package's dependencies declare in their destack.json, in name order. */
export async function loadExtensions(
    directory: string,
    declaration: PackageDescription,
): Promise<LoadedExtension[]> {
    // read the dependencies in a stable order
    const names = Object.keys({
        ...declaration.dependencies,
        ...declaration.peerDependencies,
        ...declaration.optionalDependencies,
    }).toSorted();
    const locator = new PackageLocator();
    const extensions: LoadedExtension[] = [];
    for (const name of names) {
        // skip an absent optional dependency and refuse an absent required one
        const installed = locator.directory(name, directory);
        if (installed === undefined) {
            if (isOptional(name, declaration)) {
                continue;
            }
            throw new BuildError("BUILD_FAILED", `missing dependency: ${name}`);
        }

        // skip dependencies declaring no extension
        const path = join(installed, "destack.json");
        if (!existsSync(path)) {
            continue;
        }
        const definition = PackageDefinition.read(await readFile(path, "utf8"));
        if (definition.build === undefined) {
            continue;
        }

        // import the extension from the export its build field refers to
        const extension = await importExtension(name, installed, definition.build);
        extensions.push({ directory: installed, extension });
    }

    return extensions;
}

/** Import the extension a dependency's build field names, such as `./build#viewExtension`. */
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
    const manifest = Manifest.parse(
        JSON.parse(await readFile(join(installed, "package.json"), "utf8")),
    );
    const file = selectExport(manifest.exports?.[subpath], CONDITIONS);
    if (typeof file !== "string") {
        throw new BuildError("BUILD_FAILED", `${name} exports no build module at ${subpath}`);
    }

    // read the extension from the module's exports
    const module = Exports.parse(await import(join(installed, file)));
    const extension = module[exportName];
    if (typeof extension !== "object" || extension === null) {
        throw new BuildError("BUILD_FAILED", `${name} exports no build extension ${exportName}`);
    }

    return extension;
}

/** Decide whether a package builds without a dependency, as an optional one or an optional peer. */
function isOptional(name: string, declaration: PackageDescription): boolean {
    // accept an optional dependency, which overrides a matching dependency
    if (Object.hasOwn(declaration.optionalDependencies, name)) {
        return true;
    }

    // accept an optional peer the package does not also require
    const isRequired = Object.hasOwn(declaration.dependencies, name);
    const isOptionalPeer = declaration.peerDependenciesMeta[name]?.optional === true;

    return !isRequired && isOptionalPeer;
}
