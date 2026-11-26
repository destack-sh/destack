import { Digest, present, schema } from "@destack/schema";
import type { BuildReader } from "./reader.ts";
import { DependencyName, PeerDependency } from "../definition/index.ts";
import { PackagePath } from "../file/index.ts";
import { Runtime } from "../runtime/index.ts";
import { WorkloadDescription } from "./workload.ts";
import { FrameDescription, ViewDescription } from "./view.ts";
import { DeclarationName } from "../definition/package.ts";

/** A compiled entry: its emitted file and the dependency specifiers its chunks import. */
export const PackageChunk = schema.object({
    /** The emitted file of the export. */
    path: PackagePath,
    /** The dependency specifiers the export's chunks import, such as `@destack/object/client`. */
    imports: schema.array(schema.string().min(1)),
});
/** A compiled entry: its emitted file and the dependency specifiers its chunks import. */
export type PackageChunk = schema.Infer<typeof PackageChunk>;

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
    /** The frame of a browser development build, which its origin serves at `/.destack/frame`. */
    frame: FrameDescription.exactOptional(),
    /** The output directory within the build. */
    directory: PackagePath,
    /** Generated entrypoints keyed by their exported names. */
    exports: schema.record(schema.string().min(1), PackageChunk),
    /** The peers its modules import, which each package depending on it provides beside it. */
    peers: schema.record(DependencyName, PeerDependency),
    /** The commands its package installs, each its emitted module by name, absent for none. */
    bins: schema.record(schema.string().min(1), PackageChunk).exactOptional(),
});

/** Compiled files for one execution target. */
export type PackageOutput = schema.Infer<typeof PackageOutput>;

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
    /** The paths of the modules it imports statically. */
    readonly statics: readonly string[];
    /** Whether it imports a module by a computed name, which reaches any module beside it. */
    readonly isOpen: boolean;
}

/** What a map's modules are read from: a build's manifest, its modules and a file's bytes. */
export type ModuleReader = Pick<BuildReader, "manifest" | "modules" | "load">;

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
    /** Resolve a specifier a module imports as browsers do, absent for one the map leaves. */
    resolve(map: ImportMap, specifier: string, referrer: string): string | undefined {
        return resolveAlong(chainOf(map, referrer), specifier);
    },

    /** Rewrite the specifiers a module imports to where the map resolves them, ahead of loading. */
    async apply(map: ImportMap, code: string, referrer: string): Promise<string> {
        // find each literal specifier of a static, re-exporting or dynamic import, the module's scopes read once
        const [imports] = await parseImports(code);
        const chain = chainOf(map, referrer);
        const { default: MagicString } = await import("magic-string");
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
        const [imports] = await parseImports(code);
        const chain = chainOf(map, referrer);
        const modules: string[] = [];
        const statics: string[] = [];
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
            const resolved =
                mapped ??
                (RELATIVE_SPECIFIER.test(specifier)
                    ? new URL(specifier, `${URL_BASE}${referrer}`).pathname
                    : undefined);
            if (resolved !== undefined) {
                modules.push(resolved);
            }
            if (
                resolved !== undefined &&
                (imported.type === "static" || imported.type === "reexport-star")
            ) {
                statics.push(resolved);
            }
        }

        // add each file the module names by URL beside itself, as a worker's script is
        for (const [, specifier = ""] of code.matchAll(META_URL)) {
            modules.push(new URL(specifier, `${URL_BASE}${referrer}`).pathname);
        }

        return { modules, statics, isOpen };
    },

    /** List the builds a map's URLs point into below a URL prefix, its root's own left out. */
    builds(map: ImportMap, base: string): Set<Digest> {
        // read the build digest leading each URL below the prefix
        const urls = [
            ...Object.values(map.imports),
            ...Object.values(map.scopes).flatMap((scope) => Object.values(scope)),
        ];
        const digests = urls.flatMap((url) =>
            url.startsWith(base) ? [url.slice(base.length).split("/")[0] ?? ""] : [],
        );

        return new Set(digests.map((digest) => Digest.parse(digest)));
    },

    /** Read each module some entries import through a map, directly or through others, by path. */
    async modules(
        map: ImportMap,
        base: string,
        runtime: Runtime,
        entries: readonly string[],
        contents: (build: Digest) => Promise<ModuleReader>,
    ): Promise<Map<string, string>> {
        // start at the entries, and follow what each module imports
        const read = new Map<string, string>();
        const listed = new Map<Digest, ReadonlySet<string>>();
        const pending = [...entries];
        for (let name = pending.pop(); name !== undefined; name = pending.pop()) {
            // read a module once
            if (read.has(name)) {
                continue;
            }

            // leave a path that is no module of its build on the runtime
            const [digest = "", ...path] = name.split("/");
            const build = Digest.parse(digest);
            const reader = await contents(build);
            const names = listed.get(build) ?? (await reader.modules(runtime));
            listed.set(build, names);
            if (!names.has(name)) {
                continue;
            }

            // reach each module it imports, or its whole build where it imports by a computed name
            const code = new TextDecoder().decode(await reader.load(path.join("/")));
            read.set(name, code);
            const imported = await ImportMap.imports(map, code, `${base}${name}`);
            pending.push(
                ...imported.modules.map((module) => module.slice(base.length)),
                ...(imported.isOpen ? names : []),
            );
        }

        return read;
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

/** Parse a module's imports, loading the lexer only once a caller reads a module. */
async function parseImports(
    code: string,
): Promise<ReturnType<typeof import("es-module-lexer/js").parse>> {
    const lexer = await import("es-module-lexer/js");
    await lexer.init();

    return lexer.parse(code);
}
