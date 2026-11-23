import type { BuildDirectory } from "../build/directory.ts";
import { isAbsolute, dirname, resolve } from "node:path";
import type { Environment } from "@destack/environment";
import { type Plugin } from "vite";
import type { OutputBundle } from "rolldown";
import { type ESTree, parseSync, Visitor } from "rolldown/utils";
import { PackagePath } from "@destack/package/file";
import { modulePath, type ModuleSource } from "./dependency.ts";
import { BuildError } from "../error/index.ts";
import { relativePath } from "../source/dependency.ts";
import { present, schema } from "@destack/schema";

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
    name: string,
    files: Map<string, Uint8Array<ArrayBuffer>>,
    sources: Pick<BuildDirectory, "retained" | "retain" | "copy">,
    environment: Environment<"files">,
): Plugin {
    return {
        name: "destack-source",
        enforce: "pre",
        async generateBundle(_options, bundle) {
            omitUrlSources(bundle);

            // keep the package files the kept modules import for types alone
            const resolveImport = async (specifier: string, importer: string) =>
                (await this.resolve(specifier, importer))?.id;
            await retainTypeImports(directory, name, files, sources, environment, resolveImport);
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
                    await sources.copy(PackagePath.parse(local), path);
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

/** Keep the package files kept modules import only for types, and the files those import. */
async function retainTypeImports(
    directory: string,
    name: string,
    files: Map<string, Uint8Array<ArrayBuffer>>,
    sources: Pick<BuildDirectory, "retained" | "retain">,
    environment: Environment<"files">,
    resolveImport: (specifier: string, importer: string) => Promise<string | undefined>,
): Promise<void> {
    // walk the kept TypeScript modules in waves, resolving each wave's type-only imports at once
    let wave = [...files.keys()].filter((path) => /\.[cm]?tsx?$/u.test(path));
    while (wave.length > 0) {
        // resolve every type-only import of the wave's modules to a package file
        const targets = await Promise.all(
            wave.flatMap((path) => {
                const text = new TextDecoder().decode(present(files.get(path), `source '${path}'`));

                return typeImports(path, text)
                    .filter((each) => isOwn(each, name))
                    .map(async (specifier) => {
                        const resolved = await resolveImport(specifier, resolve(directory, path));

                        return resolved === undefined
                            ? undefined
                            : packageFile(directory, modulePath(resolved));
                    });
            }),
        );

        // keep each new target once, and walk the kept ones next
        const next = new Set<string>();
        for (const target of targets) {
            if (target !== undefined && !files.has(PackagePath.parse(target))) {
                next.add(PackagePath.parse(target));
            }
        }
        await Promise.all(
            [...next].map(async (target) => {
                const bytes = await environment.files.readFile(resolve(directory, target));
                sources.retain(target, bytes);
                files.set(target, bytes);
            }),
        );
        wave = [...next];
    }
}

/**
 * The type-only specifiers of each module a process parsed, by path, with their text.
 *
 * Unchanged modules keep their text between builds, so a rebuild parses only the changed ones.
 */
const TYPE_IMPORTS = new Map<
    string,
    { readonly text: string; readonly specifiers: readonly string[] }
>();

/** List the specifiers a module imports or exports only for types, parsing each text once. */
function typeImports(path: string, text: string): readonly string[] {
    // take the specifiers of the module's text as parsed last
    const known = TYPE_IMPORTS.get(path);
    if (known?.text === text) {
        return known.specifiers;
    }

    // parse the text, replacing the module's earlier one
    const specifiers = parseTypeImports(path, text);
    TYPE_IMPORTS.set(path, { text, specifiers });

    return specifiers;
}

/** Parse the specifiers a module imports or exports only for types. */
function parseTypeImports(path: string, text: string): string[] {
    // read the module's declarations, refusing one that does not parse
    const parsed = parseSync(path, text);
    if (parsed.errors.length > 0) {
        throw BuildError.of("INVARIANT_BROKEN", `source '${path}' does not parse`);
    }

    // take type-only imports and exports, and type queries of imported modules
    const specifiers: string[] = [];
    const take = (node: ModuleDeclaration) => {
        const value = node.source?.value;
        if (typeof value === "string" && isTypeOnly(node)) {
            specifiers.push(value);
        }
    };
    new Visitor({
        ImportDeclaration: take,
        ExportNamedDeclaration: take,
        ExportAllDeclaration: take,
        TSImportType(node) {
            specifiers.push(node.source.value);
        },
    }).visit(parsed.program);

    return specifiers;
}

/** Report whether a specifier names a module of the package: relative, or by the package's name. */
function isOwn(specifier: string, name: string): boolean {
    return (
        specifier.startsWith("./") ||
        specifier.startsWith("../") ||
        specifier === name ||
        specifier.startsWith(`${name}/`)
    );
}

/** A module declaration with a source: an import, or an export from another module. */
type ModuleDeclaration =
    | ESTree.ImportDeclaration
    | ESTree.ExportNamedDeclaration
    | ESTree.ExportAllDeclaration;

/** Report whether a declaration is marked type, or every one of its specifiers is. */
function isTypeOnly(node: ModuleDeclaration): boolean {
    // take a declaration marked type
    if (
        ("importKind" in node && node.importKind === "type") ||
        ("exportKind" in node && node.exportKind === "type")
    ) {
        return true;
    }

    // else take one whose every specifier is marked type
    const named = "specifiers" in node ? node.specifiers : [];

    return (
        named.length > 0 &&
        named.every(
            (each) =>
                ("importKind" in each && each.importKind === "type") ||
                ("exportKind" in each && each.exportKind === "type"),
        )
    );
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
            throw BuildError.of(
                "INVARIANT_BROKEN",
                `source map of '${entry.fileName}' does not parse`,
            );
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
        throw BuildError.of("INVARIANT_BROKEN", `unknown source map input: '${source}'`);
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
