import { fileURLToPath } from "node:url";
import { isBuiltin } from "node:module";
import { dirname, extname, join, sep } from "node:path";
import { type Environment, type FileSystem, FileSystemError } from "@destack/environment";
import { Digest, schema } from "@destack/schema";
import { Distribution } from "@destack/update";
import { MANIFEST_FORMAT } from "@destack/package/manifest";
import { BuildError } from "../error/index.ts";

/** The entry a compiler process starts from, whose npm releases identify the compiler. */
const COMPILER_ENTRY = new URL("../main.ts", import.meta.url);

/** The version of the compiler's output and keys, bumped whenever either changes. */
export const COMPILER_VERSION = 11;

/** The version of the npm conversion's output, bumped whenever the conversion changes. */
export const CONVERSION_VERSION = 3;

/** The npm releases converting npm packages: the bundler and the CommonJS lexer. */
const CONVERSION_RELEASES: ReadonlySet<string> = new Set(["rolldown", "cjs-module-lexer"]);

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

/** The fingerprint of this process's compiler and npm conversion, which build keys read. */
export class CompilerFingerprint {
    /** The files and process the compiler is read through. */
    readonly environment: Environment<"files" | "process" | "host">;
    /** The digest of the compiler, read once. */
    #digest: Promise<Digest> | undefined;
    /** The digest of the npm conversion, read once. */
    #conversion: Promise<Digest> | undefined;
    /** The npm releases the compiler's entry loads, read once for both digests. */
    #releases: Promise<string[]> | undefined;

    /** Read the compiler through an environment. */
    private constructor(environment: Environment<"files" | "process" | "host">) {
        this.environment = environment;
    }

    /** Read the compiler through an environment. */
    static of(environment: Environment<"files" | "process" | "host">): CompilerFingerprint {
        return new CompilerFingerprint(environment);
    }

    /** Digest a file's bytes, null for a missing file. */
    async #file(path: string): Promise<string | null> {
        try {
            return await Digest.of(
                await this.environment.files.readFile(path),
                this.environment.crypto,
            );
        } catch (error) {
            if (FileSystemError.is(error, "FILE_NOT_FOUND")) {
                return null;
            }
            throw error;
        }
    }

    /** Identify the compiler, once, as its code stays loaded. */
    digest(): Promise<Digest> {
        this.#digest ??= this.#identifyCompiler();

        return this.#digest;
    }

    /** Identify the npm conversion, once, as its code stays loaded. */
    conversion(): Promise<Digest> {
        this.#conversion ??= this.#identifyConversion();

        return this.#conversion;
    }

    /** Identify the npm conversion by its executable, else Bun, its modules and their releases. */
    async #identifyConversion(): Promise<Digest> {
        // identify a release's conversion as its compiler
        if (this.environment.process.runtime.isStandalone) {
            return await this.digest();
        }

        // identify Bun, the compiler's version and the releases the conversion runs
        const compiler = await this.#compilerReleases();
        const conversion = compiler.filter((release) =>
            CONVERSION_RELEASES.has(release.slice(0, release.lastIndexOf("@"))),
        );

        return await Digest.json(
            {
                version: CONVERSION_VERSION,
                manifest: MANIFEST_FORMAT,
                bun: this.environment.process.runtime.revision,
                releases: await Digest.json(conversion, this.environment.crypto),
            },
            this.environment.crypto,
        );
    }

    /** Identify the compiler by its version and runtime, with its toolchain or its npm releases. */
    async #identifyCompiler(): Promise<Digest> {
        // identify a release's compiler by its version, its runtime's build and the toolchain manifest it ships
        const { runtime, executable } = this.environment.process;
        if (runtime.isStandalone) {
            const resources = Distribution.resources(executable);
            const manifest = Distribution.toolchainManifest(this.environment.host.architecture);

            return await Digest.json(
                {
                    version: COMPILER_VERSION,
                    bun: runtime.revision,
                    toolchain: await this.#file(join(resources, manifest)),
                },
                this.environment.crypto,
            );
        }

        // identify the compiler's version, Bun and the npm releases the compiler runs
        const compiler = await this.#compilerReleases();

        return await Digest.json(
            {
                version: COMPILER_VERSION,
                bun: runtime.revision,
                releases: await Digest.json(compiler, this.environment.crypto),
            },
            this.environment.crypto,
        );
    }

    /** Read the npm releases the compiler's entry loads, once. */
    #compilerReleases(): Promise<string[]> {
        this.#releases ??= CompilerFingerprint.releases(
            fileURLToPath(COMPILER_ENTRY),
            this.environment,
        );

        return this.#releases;
    }

    /** Read the npm releases an entry loads, as `name@version`, sorted, following its imports. */
    static async releases(entry: string, environment: Environment<"files">): Promise<string[]> {
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
                ? await CompilerFingerprint.#package(file, files)
                : undefined;
            // keep an installed release by its name and version, without reading its modules
            if (owner?.version !== undefined) {
                releases.add(`${owner.name}@${owner.version}`);
            }
            // follow a source's imports
            else {
                const text = await files.readTextFile(file);
                pending.push(...CompilerFingerprint.#imports(text, file));
            }
        }

        return [...releases].toSorted();
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
