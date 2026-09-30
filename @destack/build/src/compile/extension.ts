import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { type BuildExtension, runtimeConditions } from "@destack/package/build";
import { PackageDefinition, type PackageDescription } from "@destack/package";
import { PackageLocator } from "@destack/package/transform";
import { BuildError } from "../error/index.ts";
import { selectExport } from "../source/index.ts";

/** The conditions selecting an extension's module in the build worker. */
const CONDITIONS = new Set(["import", "default", ...runtimeConditions("bun")]);

/** Load the extensions the package's dependencies declare in their destack.json, in name order. */
export async function loadExtensions(
    directory: string,
    declaration: PackageDescription,
): Promise<BuildExtension[]> {
    // read the dependencies in a stable order
    const names = Object.keys({
        ...declaration.dependencies,
        ...declaration.peerDependencies,
        ...declaration.optionalDependencies,
    }).sort();
    const locator = new PackageLocator();
    const extensions: BuildExtension[] = [];
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
        const [subpath, exportName] = definition.build.split("#") as [string, string];
        const manifest = JSON.parse(await readFile(join(installed, "package.json"), "utf8"));
        const file = selectExport(manifest.exports?.[subpath], CONDITIONS);
        if (typeof file !== "string") {
            throw new BuildError("BUILD_FAILED", `${name} exports no build module at ${subpath}`);
        }
        const module = (await import(join(installed, file))) as Record<string, unknown>;
        const extension = module[exportName];
        if (typeof extension !== "object" || extension === null) {
            throw new BuildError(
                "BUILD_FAILED",
                `${name} exports no build extension ${exportName}`,
            );
        }
        extensions.push(extension as BuildExtension);
    }

    return extensions;
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
