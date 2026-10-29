import { isBuiltin } from "node:module";
import { dirname, extname, join, relative, sep } from "node:path";
import { type Environment, type FileSystem, FileSystemError } from "@destack/environment";
import { Digest, type DigestCrypto, schema } from "@destack/schema";
import { BuildError } from "../error/index.ts";

/** An entry's hashbang line, which Bun runs past but its scanner refuses. */
const HASHBANG = /^#!.*/u;

/** The loader Bun's transpiler reads a module with, by file extension. */
const LOADERS: Readonly<Record<string, "ts" | "tsx" | "js" | "jsx">> = {
    ".ts": "ts",
    ".mts": "ts",
    ".cts": "ts",
    ".tsx": "tsx",
    ".js": "js",
    ".mjs": "js",
    ".cjs": "js",
    ".jsx": "jsx",
};

/** The name and version of the package a module belongs to. */
const ModulePackage = schema.looseObject({ name: schema.string(), version: schema.string() });

/** The modules an entry loads, as Bun resolves its imports: workspace sources and npm releases. */
export class ModuleClosure {
    /** The identity of each module: a source's package, path and digest, or an npm release. */
    readonly modules: ReadonlyMap<string, string>;

    /** Store the identities of a closure's modules. */
    private constructor(modules: ReadonlyMap<string, string>) {
        this.modules = modules;
    }

    /** Read the modules an entry imports, sources by their bytes and npm releases by name. */
    static async read(entry: string, environment: Environment<"files">): Promise<ModuleClosure> {
        // follow each source's imports once, keying a module under node_modules by its release
        const { files, crypto } = environment;
        const modules = new Map<string, string>();
        const visited = new Set<string>();
        const pending = [entry];
        for (let file = pending.pop(); file !== undefined; file = pending.pop()) {
            if (visited.has(file)) {
                continue;
            }
            visited.add(file);
            const owner = await ModuleClosure.#package(file, files);
            const path = relative(owner.directory, file).split(sep).join("/");
            // identify an npm release by its name and version, without reading its modules
            if (file.split(sep).includes("node_modules")) {
                modules.set(`${owner.name}@${owner.version}`, "release");
            }
            // identify a source by its package, path and bytes, and follow its imports
            else {
                const bytes = await files.readFile(file);
                modules.set(`${owner.name}/${path}`, await Digest.of(bytes, crypto));
                pending.push(...ModuleClosure.#imports(new TextDecoder().decode(bytes), file));
            }
        }

        return new ModuleClosure(
            new Map([...modules].toSorted(([left], [right]) => (left < right ? -1 : 1))),
        );
    }

    /** Select the npm releases and the sources below some prefixes, leaving other sources out. */
    select(prefixes: ReadonlySet<string>): ModuleClosure {
        const isSelected = (key: string, identity: string) =>
            identity === "release" || [...prefixes].some((prefix) => key.startsWith(prefix));

        return new ModuleClosure(
            new Map([...this.modules].filter(([key, identity]) => isSelected(key, identity))),
        );
    }

    /** Select the npm releases of some packages by name, leaving every source out. */
    releases(names: ReadonlySet<string>): ModuleClosure {
        return new ModuleClosure(
            new Map(
                [...this.modules].filter(
                    ([key, identity]) =>
                        identity === "release" && names.has(key.slice(0, key.lastIndexOf("@"))),
                ),
            ),
        );
    }

    /** Digest the closure by its modules' identities. */
    async digest(crypto: DigestCrypto): Promise<Digest> {
        return await Digest.json([...this.modules], crypto);
    }

    /** Resolve the imports Bun loads of a module to files, leaving runtime modules out. */
    static #imports(text: string, file: string): string[] {
        // scan the module past its hashbang as Bun's transpiler loads it
        const loader = LOADERS[extname(file)];
        if (loader === undefined) {
            throw BuildError.of(
                "INVARIANT_VIOLATED",
                `the toolchain loads a module Bun cannot scan: ${file}`,
            );
        }
        const scanned = text.replace(HASHBANG, "");
        const imports = new Bun.Transpiler({ loader }).scanImports(scanned);

        // resolve each import but runtime modules
        return imports.flatMap(({ path: specifier }) => {
            const isRuntime =
                isBuiltin(specifier) || specifier.startsWith("bun:") || specifier === "bun";

            return isRuntime ? [] : [Bun.resolveSync(specifier, dirname(file))];
        });
    }

    /** Find the package a module belongs to: its directory, name and version. */
    static async #package(
        file: string,
        files: FileSystem,
    ): Promise<{ readonly directory: string; readonly name: string; readonly version: string }> {
        // take the nearest manifest declaring a package
        for (
            let directory = dirname(file);
            dirname(directory) !== directory;
            directory = dirname(directory)
        ) {
            const text = await files
                .readTextFile(join(directory, "package.json"))
                .catch((error: unknown) => {
                    if (FileSystemError.is(error, "NOT_FOUND")) {
                        return undefined;
                    }
                    throw error;
                });
            const parsed =
                text === undefined ? undefined : ModulePackage.safeParse(JSON.parse(text));
            if (parsed?.success === true) {
                return { directory, ...parsed.data };
            }
        }
        throw BuildError.of("DEPENDENCY_UNRESOLVED", `no package declares the module ${file}`);
    }
}
