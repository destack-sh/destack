import type { BuildDirectory } from "../build/directory.ts";
import { isAbsolute, dirname, resolve } from "node:path";
import type { Environment } from "@destack/environment";
import { type Plugin } from "vite";
import type { OutputBundle } from "rolldown";
import { PackagePath } from "@destack/package/file";
import { modulePath, type ModuleSource } from "./dependency.ts";
import { BuildError } from "../error/index.ts";
import { relativePath } from "../source/dependency.ts";
import { schema } from "@destack/schema";

/** The virtual module namespaces: plugin modules, and Vite's for absent optional peers. */
const VIRTUAL_NAMESPACES = ["virtual:", "__vite-optional-peer-dep:"];

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
    sources: Pick<BuildDirectory, "retained" | "retain">,
    destination: string,
    paths: string[],
    environment: Environment<"files">,
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
                // leave virtual modules and dependency files to the plugins that load them
                const path = modulePath(id);
                const local = packageFile(directory, path);
                if (local === undefined) {
                    return null;
                }

                // preserve large assets by file copy while Vite loads them separately
                const isModule = /\.[cm]?[jt]sx?$/u.test(path);
                if (!sources.retained.has(local) && !isModule) {
                    const output = resolve(destination, PackagePath.parse(local));
                    await environment.files.mkdir(dirname(output), { recursive: true });
                    await environment.files.copyFile(path, output);
                    paths.push(local);
                    return null;
                }

                // snapshot a module's bytes once for every output, refusing one changed between them
                const bytes =
                    sources.retained.get(local) ?? (await environment.files.readFile(path));
                if (isModule) {
                    sources.retain(local, bytes);
                }
                files.set(PackagePath.parse(local), bytes);
                if (!id.includes("?") && isModule) {
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
            throw BuildError.of("INVARIANT_VIOLATED", `invalid source map: ${entry.fileName}`);
        }

        // preserve source indices and mappings for generated URL modules
        for (const [index, source] of map.sources.entries()) {
            if (/\?(?:[^#]*&)?url(?:&|$)/u.test(source) && map.sourcesContent) {
                map.sourcesContent[index] = null;
            }
        }
        entry.source = JSON.stringify(map);
    }
}

/** Return a module path relative to the package directory, if inside the package. */
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
    // preserve the identifiers of virtual modules, which rolldown writes as paths
    for (const namespace of VIRTUAL_NAMESPACES) {
        const virtual = source.indexOf(namespace);
        if (virtual !== -1) {
            return source.slice(virtual);
        }
    }

    // use the same package attribution as compiler inspection
    const absolute = resolve(dirname(map), source);
    const location = locations.get(absolute);
    if (!location) {
        throw BuildError.of("INVARIANT_VIOLATED", `unknown source map input: ${source}`);
    }
    if (location.package !== undefined) {
        return `package:${location.package}/${location.path}`;
    }

    return relativePath(dirname(output), location.path);
}

/** Report whether a value is a source map, preserving the parsed object and its key order. */
function isSourceMap(value: unknown): value is SourceMap {
    return SourceMap.safeParse(value).success;
}
