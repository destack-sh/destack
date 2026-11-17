import * as lexer from "es-module-lexer/js";
import MagicString from "magic-string";
import { present, schema } from "@destack/schema";
import { DependencyName } from "../definition/index.ts";
import { PackageFile, PackagePath } from "../file/index.ts";
import { Runtime } from "../runtime/index.ts";
import { WorkloadDescription } from "./workload.ts";
import { ViewDescription } from "./view.ts";
import { DeclarationName } from "../definition/package.ts";

/** An output's export: its emitted file and the dependency specifiers its chunks import. */
export const OutputExport = schema.object({
    /** The emitted file of the export. */
    path: PackagePath,
    /** The dependency specifiers the export's chunks import, such as `@destack/object/client`. */
    imports: schema.array(schema.string().min(1)),
});
/** An output's export: its emitted file and the dependency specifiers its chunks import. */
export type OutputExport = schema.Infer<typeof OutputExport>;

/** A peer an output's package declares, which its dependents provide. */
export const OutputPeer = schema.object({
    /** Whether a dependent may leave it out, as npm's peerDependenciesMeta marks it. */
    isOptional: schema.boolean(),
});

/** A peer an output's package declares, which its dependents provide. */
export type OutputPeer = schema.Infer<typeof OutputPeer>;

/** Compiled files and dependencies for one runtime. */
export const PackageOutput = schema.object({
    /** The runtime the output is compiled for. */
    runtime: Runtime,
    /** Whether this output is distributed for execution. */
    emit: schema.boolean(),
    /** Validated workloads keyed by their declared names. */
    workloads: schema.record(DeclarationName, WorkloadDescription),
    /** The views this output mounts, keyed by their declared names. */
    views: schema.record(DeclarationName, ViewDescription),
    /** The output directory within the build. */
    directory: PackagePath,
    /** Generated entrypoints keyed by their exported names. */
    exports: schema.record(schema.string().min(1), OutputExport),
    /** The peers its package declares, which each package depending on it provides beside it. */
    peers: schema.record(DependencyName, OutputPeer).default({}),
    /** The commands its package installs, each its emitted module by name, absent for none. */
    bins: schema.record(schema.string().min(1), OutputExport).exactOptional(),
    /** The list of the files its runtime loads: its own, the package.json and the catalogs. */
    files: PackageFile,
});

/** Compiled files for one execution target. */
export type PackageOutput = schema.Infer<typeof PackageOutput>;

/** An output as a build writes it, before the writer lists the files its runtime loads. */
export const OutputContent = PackageOutput.omit({ files: true });
/** An output as a build writes it, before the writer lists the files its runtime loads. */
export type OutputContent = schema.Input<typeof OutputContent>;

/** A specifier a module resolves against its own path: one starting at a directory or the root. */
const RELATIVE_SPECIFIER = /^(?:\.{1,2})?\//u;

/** The start of an import expression, which a module may give a computed name. */
const IMPORT_CALL = /^import\s*\(/u;

/** The characters read at an import to tell an import expression: the word, spaces, a bracket. */
const IMPORT_CALL_LENGTH = 16;

/** A file a module names by URL against its own, as a worker's script is named. */
const META_URL = /new URL\(\s*["'`]([^"'`]+)["'`]\s*,\s*import\.meta\.url\s*\)/gu;

/** The origin a module's path resolves a relative specifier under, dropped from the result. */
const URL_BASE = "file://";

/** What a module imports: the modules it reaches, and whether it imports one by a computed name. */
export interface ModuleImports {
    /** The paths of the modules its literal imports resolve to. */
    readonly modules: readonly string[];
    /** Whether it imports a module by a computed name, which reaches any module beside it. */
    readonly isOpen: boolean;
}

/** An import map, as browsers read it: module specifiers to URLs, with scopes by URL prefix. */
const importMap = schema.object({
    /** The URL of each specifier the page's own modules import. */
    imports: schema.record(schema.string().min(1), schema.string()),
    /** The URL of each specifier the modules below a URL prefix import, by prefix. */
    scopes: schema.record(
        schema.string().min(1),
        schema.record(schema.string().min(1), schema.string()),
    ),
});
/** An import map, as browsers read it: module specifiers to URLs, with scopes by URL prefix. */
export const ImportMap = Object.assign(importMap, {
    /** Write the script element of an import map, escaped for HTML. */
    script(map: ImportMap): string {
        return `<script type="importmap">${JSON.stringify(map).replaceAll("<", "\\u003c")}</script>`;
    },

    /** Insert an import map at the start of a page's head, before every module it loads. */
    insert(html: string, map: ImportMap): string {
        return html.replace(/<head(?:\s[^>]*)?>/u, (head) => `${head}${ImportMap.script(map)}`);
    },

    /** Resolve a specifier a module imports as browsers do, absent for one the map leaves. */
    resolve(map: ImportMap, specifier: string, referrer: string): string | undefined {
        return resolveAlong(chainOf(map, referrer), specifier);
    },

    /** Rewrite the specifiers a module imports to where the map resolves them, ahead of loading. */
    async apply(map: ImportMap, code: string, referrer: string): Promise<string> {
        // find each literal specifier of a static, re-exporting or dynamic import, the module's scopes read once
        await lexer.init();
        const [imports] = lexer.parse(code);
        const chain = chainOf(map, referrer);
        const rewritten = new MagicString(code);
        for (const imported of imports) {
            const { specifier } = imported;
            const isLiteral =
                typeof specifier === "string" && !("glob" in imported && imported.glob);
            const resolved = isLiteral ? resolveAlong(chain, specifier) : undefined;

            // replace a mapped one, keeping the quotes a dynamic import's span may include
            if (resolved !== undefined) {
                const isQuoted = /["'`]/u.test(code[imported.start] ?? "");
                const text = isQuoted ? JSON.stringify(resolved) : resolved;
                rewritten.overwrite(imported.start, imported.end, text);
            }
        }

        return rewritten.toString();
    },

    /** List the modules a module imports as the map and its own path resolve them. */
    async imports(map: ImportMap, code: string, referrer: string): Promise<ModuleImports> {
        // resolve each literal specifier through the map, a relative one against the module
        await lexer.init();
        const [imports] = lexer.parse(code);
        const chain = chainOf(map, referrer);
        const modules: string[] = [];
        let isOpen = false;
        for (const imported of imports) {
            const { specifier } = imported;
            const isLiteral =
                typeof specifier === "string" && !("glob" in imported && imported.glob);
            if (!isLiteral) {
                // leave an import of a computed name open, as no reader can list what it loads
                isOpen ||= IMPORT_CALL.test(
                    code.slice(imported.importStart, imported.importStart + IMPORT_CALL_LENGTH),
                );
                continue;
            }
            const mapped = resolveAlong(chain, specifier);
            if (mapped !== undefined) {
                modules.push(mapped);
            } else if (RELATIVE_SPECIFIER.test(specifier)) {
                modules.push(new URL(specifier, `${URL_BASE}${referrer}`).pathname);
            }
        }

        // add each file the module names by URL beside itself, as a worker's script is
        for (const [, specifier = ""] of code.matchAll(META_URL)) {
            modules.push(new URL(specifier, `${URL_BASE}${referrer}`).pathname);
        }

        return { modules, isOpen };
    },
});
/** An import map, as browsers read it: module specifiers to URLs, with scopes by URL prefix. */
export type ImportMap = schema.Infer<typeof importMap>;

/** List the maps a module resolves along: its scopes, most specific first, then the imports. */
function chainOf(map: ImportMap, referrer: string): Readonly<Record<string, string>>[] {
    const scopes = Object.keys(map.scopes)
        .filter((prefix) => referrer.startsWith(prefix))
        .toSorted((left, right) => right.length - left.length)
        .map((prefix) => map.scopes[prefix] ?? {});

    return [...scopes, map.imports];
}

/** Resolve a specifier along a chain of specifier maps, absent where none maps it. */
function resolveAlong(
    chain: readonly Readonly<Record<string, string>>[],
    specifier: string,
): string | undefined {
    // take the first map resolving it
    for (const specifiers of chain) {
        const resolved = resolveIn(specifiers, specifier);
        if (resolved !== undefined) {
            return resolved;
        }
    }

    return undefined;
}

/** Resolve a specifier in one specifier map: its exact entry, else its longest slash prefix. */
function resolveIn(
    specifiers: Readonly<Record<string, string>>,
    specifier: string,
): string | undefined {
    // take the longest key ending in a slash that starts the specifier, unless one matches exactly
    const exact = specifiers[specifier];
    const prefix = Object.keys(specifiers)
        .filter((key) => key.endsWith("/") && specifier.startsWith(key))
        .toSorted((left, right) => right.length - left.length)[0];

    // answer the exact entry, else the prefix's target with the rest of the specifier
    if (exact !== undefined) {
        return exact;
    } else if (prefix !== undefined) {
        return `${present(specifiers[prefix], prefix)}${specifier.slice(prefix.length)}`;
    } else {
        return undefined;
    }
}
