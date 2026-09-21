import { readFile } from "node:fs/promises";
import { isAbsolute, relative, sep, dirname, resolve } from "node:path";
import { type Plugin } from "vite";
import { PackagePath } from "@destack/package/file";
import { type ModuleSource } from "./dependency.ts";
import { BuildError } from "../error/index.ts";
import { transform } from "rolldown/utils";
import { ModuleMetadata } from "@destack/package/package";
import { type Package } from "@destack/package";
import { modulePackage } from "../source/dependency.ts";

/** Retain authored files read by Vite. */
export function sourcePlugin(
    directory: string,
    files: Map<string, Uint8Array<ArrayBuffer>>,
    sources: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
): Plugin {
    return {
        name: "destack-source",
        enforce: "pre",
        load: {
            filter: { id: new RegExp(`^${RegExp.escape(directory.replaceAll("\\", "/"))}/`) },
            async handler(id) {
                // leave virtual modules and dependency files to their owning plugins
                const path = id.split("?")[0];
                if (!isAbsolute(path)) {
                    return;
                }
                const local = relative(directory, path).split(sep).join("/");
                if (
                    local.startsWith("../") ||
                    isAbsolute(local) ||
                    local.split("/").includes("node_modules")
                ) {
                    return;
                }

                // compile inspected module bytes and retain additional assets read by Vite
                if (/\.[cm]?tsx?$/.test(path) && !sources.has(local)) {
                    throw new BuildError("BUILD_FAILED", `uninspected source module: ${local}`);
                }
                const bytes = sources.get(local) ?? new Uint8Array(await readFile(path));
                files.set(PackagePath.parse(local), bytes);
                if (!id.includes("?") && /\.[cm]?[jt]sx?$/.test(path)) {
                    return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
                }
            },
        },
    };
}

/** Map compiler sources to retained files or package-qualified dependency sources. */
export function mapSource(
    source: string,
    map: string,
    output: string,
    locations: ReadonlyMap<string, ModuleSource>,
): string {
    // preserve virtual module identifiers emitted by framework plugins
    const virtual = source.indexOf("virtual:");
    if (virtual !== -1) {
        return source.slice(virtual);
    }

    // use the same package attribution as compiler inspection
    const absolute = resolve(dirname(map), source);
    const location = locations.get(absolute);
    if (!location) {
        throw new BuildError("BUILD_FAILED", `Unknown source map input: ${source}`);
    }
    if (location.package) {
        return `package:${location.package}/${location.path}`;
    }

    return relative(dirname(output), location.path).split(sep).join("/");
}

/** Replace module metadata before bundling and preserve the transformation's source map. */
export function metadataPlugin(directory: string, source: Package, configuration: string): Plugin {
    const metadata = ModuleMetadata.parse({ package: source });

    return {
        name: "destack-module-metadata",
        transform: {
            filter: { id: /\.[cm]?[jt]sx?$/, code: /import\.meta\.destack/ },
            async handler(code, id) {
                // leave bundler-generated virtual modules outside package metadata injection
                if (id.startsWith("\0")) {
                    return;
                }
                const path = relative(directory, id);
                const external =
                    isAbsolute(path) ||
                    path === ".." ||
                    path.startsWith(`..${sep}`) ||
                    path.split(sep).includes("node_modules");
                const owner = external ? await modulePackage(dirname(id)) : source;
                const identity = external
                    ? ModuleMetadata.parse({
                          package: { name: owner.name, version: owner.version },
                      })
                    : metadata;

                // replace each module's metadata before code from multiple modules is combined
                const result = await transform(id, code, {
                    tsconfig: configuration,
                    jsx: "preserve",
                    sourcemap: true,
                    define: { "import.meta.destack": JSON.stringify(identity) },
                });
                if (result.errors.length) {
                    throw new BuildError(
                        "BUILD_FAILED",
                        `Module transformation failed: ${JSON.stringify(result.errors)}`,
                    );
                }

                return { code: result.code, map: result.map };
            },
        },
    };
}
