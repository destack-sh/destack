import { readFile, mkdir, copyFile } from "node:fs/promises";
import { isAbsolute, dirname, resolve } from "node:path";
import { type Plugin } from "vite";
import type { OutputBundle } from "rolldown";
import { PackagePath } from "@destack/package/file";
import { modulePath, type ModuleSource } from "./dependency.ts";
import { BuildError } from "../error/index.ts";
import { relativePath } from "../source/dependency.ts";
import { schema } from "@destack/schema";

/** The sources of a source map and their contents. */
const SourceMap = schema.looseObject({
    sources: schema.array(schema.string()),
    sourcesContent: schema.array(schema.string().nullable()).exactOptional(),
});
/** The sources of a source map and their contents. */
type SourceMap = schema.Infer<typeof SourceMap>;

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
            omitUrlSources(bundle);
        },
        load: {
            filter: { id: new RegExp(`^${RegExp.escape(directory.replaceAll("\\", "/"))}/`, "u") },
            async handler(id) {
                // leave virtual modules and dependency files to their owning plugins
                const path = modulePath(id);
                const local = packageFile(directory, path);
                if (local === undefined) {
                    return null;
                }

                // compile inspected module bytes and retain additional assets read by Vite
                if (/\.[cm]?tsx?$/u.test(path) && !sources.has(local)) {
                    throw new BuildError("BUILD_FAILED", `uninspected source module: ${local}`);
                }

                // preserve large assets by file copy while Vite loads them separately
                if (!sources.has(local) && !/\.[cm]?[jt]sx?$/u.test(path)) {
                    const output = resolve(destination, PackagePath.parse(local));
                    await mkdir(dirname(output), { recursive: true });
                    await copyFile(path, output);
                    paths.push(local);
                    return null;
                }

                // return inspected source to the compiler without rereading it
                const bytes = sources.get(local) ?? new Uint8Array(await readFile(path));
                files.set(PackagePath.parse(local), bytes);
                if (!id.includes("?") && /\.[cm]?[jt]sx?$/u.test(path)) {
                    return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
                }

                return null;
            },
        },
    };
}

/** Clear the source content of generated URL modules in each emitted source map. */
function omitUrlSources(bundle: OutputBundle): void {
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
        const map: unknown = JSON.parse(text);
        if (!isSourceMap(map)) {
            throw new BuildError("BUILD_FAILED", `invalid source map: ${entry.fileName}`);
        }

        // keep source indices and mappings intact for generated URL modules
        for (const [index, source] of map.sources.entries()) {
            if (/\?(?:[^#]*&)?url(?:&|$)/u.test(source) && map.sourcesContent) {
                map.sourcesContent[index] = null;
            }
        }
        entry.source = JSON.stringify(map);
    }
}

/** Return a module path relative to the package directory, or nothing outside the package's own files. */
function packageFile(directory: string, path: string): string | undefined {
    // skip virtual modules
    if (!isAbsolute(path)) {
        return undefined;
    }

    // skip files outside the package and dependency files
    const local = relativePath(directory, path);
    if (local.startsWith("../") || isAbsolute(local) || local.split("/").includes("node_modules")) {
        return undefined;
    }

    return local;
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
        throw new BuildError("BUILD_FAILED", `unknown source map input: ${source}`);
    }
    if (location.package !== undefined) {
        return `package:${location.package}/${location.path}`;
    }

    return relativePath(dirname(output), location.path);
}

/** Report whether a value is a source map, keeping the parsed object and its key order. */
function isSourceMap(value: unknown): value is SourceMap {
    return SourceMap.safeParse(value).success;
}
