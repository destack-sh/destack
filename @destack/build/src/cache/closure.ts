import { isBuiltin } from "node:module";
import { dirname, extname, join, sep } from "node:path";
import { type Environment, type FileSystem, FileSystemError } from "@destack/environment";
import { Digest, type DigestCrypto, schema } from "@destack/schema";
import { BuildError } from "../error/index.ts";

/** An entry's hashbang line, which Bun runs past but its scanner refuses. */
const HASHBANG = /^#!.*/u;

/** The extensions Bun loads as data, whose modules import nothing, as its loaders define them. */
const DATA_EXTENSIONS: ReadonlySet<string> = new Set([
    ".json",
    ".jsonc",
    ".toml",
    ".yaml",
    ".yml",
    ".txt",
]);

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

/** The name of the package a module belongs to, and the version a released one has. */
const ModulePackage = schema.looseObject({
    name: schema.string(),
    version: schema.string().exactOptional(),
});

/** The npm releases an entry loads, as Bun resolves its imports through workspace sources. */
export class ModuleClosure {
    /** The releases, as `name@version`, sorted. */
    readonly releases: readonly string[];

    /** Keep the releases of a closure. */
    private constructor(releases: readonly string[]) {
        this.releases = releases;
    }

    /** Read the npm releases an entry loads, following the imports of the sources it reaches. */
    static async read(entry: string, environment: Environment<"files">): Promise<ModuleClosure> {
        // follow each source's imports once, keeping a module under node_modules as its release
        const { files } = environment;
        const releases = new Set<string>();
        const visited = new Set<string>();
        const pending = [entry];
        for (let file = pending.pop(); file !== undefined; file = pending.pop()) {
            if (visited.has(file)) {
                continue;
            }
            visited.add(file);
            const owner = file.split(sep).includes("node_modules")
                ? await ModuleClosure.#package(file, files)
                : undefined;
            // keep an installed release by its name and version, without reading its modules
            if (owner?.version !== undefined) {
                releases.add(`${owner.name}@${owner.version}`);
            }
            // follow a source's imports
            else {
                const text = await files.readTextFile(file);
                pending.push(...ModuleClosure.#imports(text, file));
            }
        }

        return new ModuleClosure([...releases].toSorted());
    }

    /** Select the releases of some packages by name. */
    select(names: ReadonlySet<string>): ModuleClosure {
        return new ModuleClosure(
            this.releases.filter((release) =>
                names.has(release.slice(0, release.lastIndexOf("@"))),
            ),
        );
    }

    /** Digest the releases. */
    async digest(crypto: DigestCrypto): Promise<Digest> {
        return await Digest.json(this.releases, crypto);
    }

    /** Resolve the imports Bun loads of a module to files, leaving runtime modules out. */
    static #imports(text: string, file: string): string[] {
        // read a data module as importing nothing
        if (DATA_EXTENSIONS.has(extname(file))) {
            return [];
        }

        // scan the module past its hashbang as Bun's transpiler loads it
        const loader = LOADERS[extname(file)];
        if (loader === undefined) {
            throw BuildError.of(
                "INVARIANT_BROKEN",
                `the toolchain loads a module Bun cannot scan: '${file}'`,
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

    /** Find the package a module belongs to: its name and a release's version. */
    static async #package(
        file: string,
        files: FileSystem,
    ): Promise<{ readonly name: string; readonly version?: string }> {
        // take the nearest manifest declaring a package
        for (
            let directory = dirname(file);
            dirname(directory) !== directory;
            directory = dirname(directory)
        ) {
            const text = await files
                .readTextFile(join(directory, "package.json"))
                .catch((error: unknown) => {
                    if (FileSystemError.is(error, "FILE_NOT_FOUND")) {
                        return undefined;
                    }
                    throw error;
                });
            const parsed =
                text === undefined ? undefined : ModulePackage.safeParse(JSON.parse(text));
            if (parsed?.success === true) {
                return parsed.data;
            }
        }
        throw BuildError.of(
            "DEPENDENCY_INVALID",
            `module '${file}' is invalid: no package declares it`,
        );
    }
}
