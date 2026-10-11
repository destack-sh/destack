import { readFile, stat } from "node:fs/promises";
import { join, relative, sep } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { Digest } from "@destack/schema";
import { isMissing } from "../error/index.ts";
import type { ParsedConfiguration } from "../typescript/index.ts";
import type { Identities } from "./cache.ts";

/** The extensions of the modules a compiler reads, which a scan follows. */
const MODULE_EXTENSIONS = [".ts", ".tsx", ".mts", ".cts"];

/** The JavaScript extensions a compiler maps to the TypeScript module beside them. */
const MAPPED_EXTENSIONS: Readonly<Record<string, readonly string[]>> = {
    ".js": [".ts", ".tsx"],
    ".jsx": [".tsx"],
    ".mjs": [".mts"],
    ".cjs": [".cts"],
};

/** A module specifier after `from`, a bare `import` or `import(`, type-only imports and re-exports among them. */
const SPECIFIER_PATTERN = /(?:\bfrom\s*|\bimport\s*\(\s*|\bimport\s+)(["'])([^"'\n]+)\1/gu;

/** A literal directory a module reads beside itself, as `new URL("./dir/", import.meta.url)` gives it. */
const DIRECTORY_PATTERN =
    /new\s+URL\(\s*(["'`])(\.{1,2}\/[^"'`$\n]*\/)\1\s*,\s*import\.meta\.url\s*\)/gu;

/** What one import of a module resolves to. */
export type ImportTarget =
    /** One of the package's modules, by absolute path. */
    | { readonly kind: "module"; readonly file: string }
    /** A file the bundler reads and the compiler does not, such as a stylesheet, by absolute path. */
    | { readonly kind: "file"; readonly file: string }
    /** A dependency, compiler library, runtime builtin or virtual module, kept by its specifier alone. */
    | { readonly kind: "external" };

/** One of the package's modules: its bytes, its imports and the directories it reads. */
export interface ModuleImports {
    /** The SHA-256 digest of the module's bytes. */
    readonly digest: Digest;
    /** The module's imports, in source order. */
    readonly imports: readonly { readonly specifier: string; readonly target: ImportTarget }[];
    /** The absolute directories the module reads through literal directory URLs. */
    readonly directories: readonly string[];
}

/** A package's program as a cache key sees it: its options, its packages and the package's modules. */
export interface ProgramImports {
    /** The effective compiler options as JSON, without the location of the generated configuration. */
    readonly options: string;
    /** The directories of the packages installed for the program, whose closures the keys identify. */
    readonly packages: readonly string[];
    /** The package's modules, by absolute path. */
    readonly modules: ReadonlyMap<string, ModuleImports>;
}

/** The scan of a package's program for its cache keys. */
export const ProgramImports = {
    /**
     * Read a package's program as cache keys see it, without loading the program: its options, its packages and the modules its root files import.
     *
     * The scan over-approximates what a compiler reads: every specifier it finds counts, type-only ones and ones in comments among them.
     */
    async scan(
        directory: string,
        configuration: ParsedConfiguration,
        identities: Identities,
    ): Promise<ProgramImports> {
        // identify the options without the location of the generated configuration
        const options = Object.entries(configuration.options).filter(
            ([key]) => key !== "configFilePath",
        );

        // list the installed packages and read the subpath imports
        const packages = await identities.installed(directory);
        const resolver = new SpecifierResolver(directory, await identities.subpaths(directory));

        // scan the root files and the modules they import within the package, a wave at a time
        const modules = new Map<string, ModuleImports>();
        let wave = configuration.files.filter((file) => isWithin(directory, file));
        while (wave.length > 0) {
            const scanned = await Promise.all(
                wave.map(async (file) => [file, await resolver.scan(file)] as const),
            );
            for (const [file, imports] of scanned) {
                modules.set(file, imports);
            }
            const next = scanned.flatMap(([, imports]) =>
                imports.imports.flatMap(({ target }) =>
                    target.kind === "module" && !modules.has(target.file) ? [target.file] : [],
                ),
            );
            wave = [...new Set(next)];
        }

        return {
            options: JSON.stringify(Object.fromEntries(options)),
            packages: [...packages].toSorted(),
            modules,
        };
    },
};

/** The resolver of a package's specifiers as its compiler resolves them, checking each path once. */
class SpecifierResolver {
    /** The package directory. */
    readonly #root: string;
    /** The package's subpath imports, each `#name` or `#prefix/*` with the files its conditions map to. */
    readonly #subpaths: ReadonlyMap<string, readonly string[]>;
    /** Whether each checked path is a file, by path. */
    readonly #files = new Map<string, Promise<boolean>>();

    /** Resolve within a package and its subpath imports. */
    constructor(root: string, subpaths: ReadonlyMap<string, readonly string[]>) {
        this.#root = root;
        this.#subpaths = subpaths;
    }

    /** Read one module: its bytes' digest, each specifier resolved within the package, and the directories it reads. */
    async scan(file: string): Promise<ModuleImports> {
        // read the module and find its specifiers and directory references
        const bytes = new Uint8Array(await readFile(file));
        const text = new TextDecoder().decode(bytes);
        const specifiers = [...text.matchAll(SPECIFIER_PATTERN)].flatMap((match) =>
            match[2] === undefined ? [] : [match[2]],
        );
        const directories = [...text.matchAll(DIRECTORY_PATTERN)].flatMap((match) =>
            match[2] === undefined ? [] : [fileURLToPath(new URL(match[2], pathToFileURL(file)))],
        );

        // resolve each specifier to its targets: a subpath import to every file it maps to
        const imports = await Promise.all(
            specifiers.flatMap((specifier) =>
                this.#mapped(specifier).map(async (mapped) => ({
                    specifier,
                    target: await this.#resolve(mapped, file),
                })),
            ),
        );

        return { digest: await Digest.of(bytes), imports, directories };
    }

    /** Map a subpath import to the absolute files of its conditions, and keep any other specifier. */
    #mapped(specifier: string): readonly string[] {
        // map an exact subpath import
        const exact = this.#subpaths.get(specifier);
        if (exact !== undefined) {
            return exact;
        }

        // map a wildcard subpath import by its prefix
        const wildcard = [...this.#subpaths].find(
            ([name]) => name.endsWith("*") && specifier.startsWith(name.slice(0, -1)),
        );
        if (wildcard !== undefined) {
            const [name, files] = wildcard;
            const rest = specifier.slice(name.length - 1);

            return files.map((file) => file.replace("*", rest));
        }

        return [specifier];
    }

    /** Resolve a specifier from a module: a module of the package, another file it reads, or something external. */
    async #resolve(specifier: string, from: string): Promise<ImportTarget> {
        // keep a bare specifier, a builtin or a dependency, external
        const isRelative = specifier.startsWith("./") || specifier.startsWith("../");
        if (!isRelative && !specifier.startsWith("/")) {
            return { kind: "external" };
        }

        // resolve the path against its module without its query, and take its first module that exists
        const path = fileURLToPath(new URL(specifier, pathToFileURL(from)));
        if (isWithin(this.#root, path)) {
            for (const candidate of moduleCandidates(path)) {
                if (await this.#isFile(candidate)) {
                    return { kind: "module", file: candidate };
                }
            }
        }

        return { kind: "file", file: path };
    }

    /** Report whether a path is a file, checking it once and refusing any failure but its absence. */
    #isFile(path: string): Promise<boolean> {
        let checked = this.#files.get(path);
        if (checked === undefined) {
            checked = stat(path).then(
                (found) => found.isFile(),
                (error: unknown) => {
                    // read a missing path, or one below a file, as no file
                    const isBelowFile =
                        error instanceof Error && "code" in error && error.code === "ENOTDIR";
                    if (isMissing(error) || isBelowFile) {
                        return false;
                    }
                    throw error;
                },
            );
            this.#files.set(path, checked);
        }

        return checked;
    }
}

/** List the modules a compiler tries for a path, in its order: the path, its TypeScript twin, an extensionless module, a directory's index. */
function moduleCandidates(path: string): string[] {
    // take a TypeScript module as it is
    if (MODULE_EXTENSIONS.some((extension) => path.endsWith(extension))) {
        return [path];
    }

    // map a JavaScript extension to the TypeScript module beside it
    const mapped = Object.entries(MAPPED_EXTENSIONS).find(([extension]) =>
        path.endsWith(extension),
    );
    if (mapped !== undefined) {
        const [extension, targets] = mapped;

        return targets.map((target) => `${path.slice(0, -extension.length)}${target}`);
    }

    // try an extensionless module, then a directory's index
    return [
        ...MODULE_EXTENSIONS.map((extension) => `${path}${extension}`),
        ...MODULE_EXTENSIONS.map((extension) => join(path, `index${extension}`)),
    ];
}

/** Report whether a file lies within a package directory and outside its installed dependencies. */
function isWithin(root: string, file: string): boolean {
    const path = relative(root, file);

    return !path.startsWith("..") && !path.split(sep).includes("node_modules");
}
