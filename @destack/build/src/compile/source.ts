import { readFile, mkdir, copyFile } from "node:fs/promises";
import { isAbsolute, relative, sep, dirname, resolve } from "node:path";
import { type Plugin } from "vite";
import { PackagePath } from "@destack/package/file";
import { type ModuleSource } from "./dependency.ts";
import { BuildError } from "../error/index.ts";

/** Retain authored files read by Vite. */
export function sourcePlugin(
    directory: string,
    files: Map<string, Uint8Array<ArrayBuffer>>,
    sources: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
    destination: string,
    paths: string[],
): Plugin {
    return {
        name: "destack-source",
        enforce: "pre",
        generateBundle(_options, bundle) {
            // omit generated asset modules containing build-local file identifiers
            for (const entry of Object.values(bundle)) {
                if (entry.type !== "asset" || !entry.fileName.endsWith(".map")) {
                    continue;
                }

                // read the emitted map before retaining its authored source content
                const text =
                    typeof entry.source === "string"
                        ? entry.source
                        : new TextDecoder().decode(entry.source);
                const map: { sources: string[]; sourcesContent?: (string | null)[] } =
                    JSON.parse(text);

                // keep source indices and mappings intact for generated URL modules
                for (const [index, source] of map.sources.entries()) {
                    if (/\?(?:[^#]*&)?url(?:&|$)/.test(source) && map.sourcesContent) {
                        map.sourcesContent[index] = null;
                    }
                }
                entry.source = JSON.stringify(map);
            }
        },
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

                // preserve large assets by file copy while Vite performs its own loading
                if (!sources.has(local) && !/\.[cm]?[jt]sx?$/.test(path)) {
                    const output = resolve(destination, PackagePath.parse(local));
                    await mkdir(dirname(output), { recursive: true });
                    await copyFile(path, output);
                    paths.push(local);
                    return;
                }

                // return inspected source to the compiler without rereading it
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
