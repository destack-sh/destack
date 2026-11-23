import { fileURLToPath } from "node:url";
import { type Environment, FileSystemError } from "@destack/environment";
import { Digest } from "@destack/schema";
import { Distribution } from "@destack/update";
import { join } from "node:path";
import { ModuleClosure } from "./closure.ts";

/** The entry a compiler process starts from, whose npm releases identify the compiler. */
const COMPILER_ENTRY = new URL("../main.ts", import.meta.url);

/** The version of the compiler's output and keys, bumped whenever either changes. */
export const COMPILER_VERSION = 3;

/** The npm releases converting npm packages: the bundler and the CommonJS lexer. */
const CONVERSION_RELEASES: ReadonlySet<string> = new Set(["rolldown", "cjs-module-lexer"]);

/** The fingerprint of this process's compiler and npm conversion, which build keys read. */
export class CompilerFingerprint {
    /** The files and process the compiler is read through. */
    readonly environment: Environment<"files" | "process" | "host">;
    /** The digest of the compiler, read once. */
    #digest: Promise<Digest> | undefined;
    /** The digest of the npm conversion, read once. */
    #conversion: Promise<Digest> | undefined;

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
        const compiler = await ModuleClosure.read(fileURLToPath(COMPILER_ENTRY), this.environment);

        return await Digest.json(
            {
                version: COMPILER_VERSION,
                bun: this.environment.process.runtime.revision,
                releases: await compiler
                    .select(CONVERSION_RELEASES)
                    .digest(this.environment.crypto),
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
        const compiler = await ModuleClosure.read(fileURLToPath(COMPILER_ENTRY), this.environment);

        return await Digest.json(
            {
                version: COMPILER_VERSION,
                bun: runtime.revision,
                releases: await compiler.digest(this.environment.crypto),
            },
            this.environment.crypto,
        );
    }
}
