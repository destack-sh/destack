import { fileURLToPath } from "node:url";
import { version } from "typescript";
import { type Environment, FileSystemError } from "@destack/environment";
import { Digest } from "@destack/schema";
import type { Toolchain } from "@destack/check/toolchain";
import { ModuleClosure } from "./closure.ts";
import { BuildError } from "../error/index.ts";

/** The entry a compiler process starts from, whose module closure identifies the compiler. */
const COMPILER_ENTRY = new URL("../main.ts", import.meta.url);

/** The layout of cache keys, changed whenever keys derive from different inputs. */
const KEY_LAYOUT = 4;

/** The workspace sources that shape build output and keys: the compiler, digests and catalogs. */
const COMPILER_SOURCES: ReadonlySet<string> = new Set([
    "@destack/build/",
    "@destack/package/",
    "@destack/schema/src/digest/",
    "@destack/schema/src/json/",
    "@destack/locale/src/message/catalog.ts",
]);

/** The modules converting npm packages, whose bytes shape an npm unit's build. */
const CONVERSION_MODULES = [
    new URL("../compile/conversion.ts", import.meta.url),
    new URL("../dependency/npm.ts", import.meta.url),
];

/** The writer of build manifests, whose module closure shapes every manifest's bytes. */
const MANIFEST_WRITER = new URL("../../../package/src/manifest/writer.ts", import.meta.url);

/** The npm releases converting npm packages: the bundler and the CommonJS lexer. */
const CONVERSION_RELEASES: ReadonlySet<string> = new Set(["rolldown", "cjs-module-lexer"]);

/** The identity of this process's compiler and npm conversion, which build keys read. */
export class CompilerIdentity {
    /** The files and process the compiler is read through. */
    readonly environment: Environment<"files" | "process">;
    /** Read the tools the compiler runs, whose manifest a release's identity includes. */
    readonly toolchain: () => Promise<Toolchain>;
    /** The digest of the compiler, read once. */
    #digest: Promise<Digest> | undefined;
    /** The digest of the npm conversion, read once. */
    #conversion: Promise<Digest> | undefined;

    /** Read the compiler through an environment, running a toolchain read once it is needed. */
    private constructor(
        environment: Environment<"files" | "process">,
        toolchain: () => Promise<Toolchain>,
    ) {
        this.environment = environment;
        this.toolchain = toolchain;
    }

    /** Read the compiler through an environment, running a toolchain read once it is needed. */
    static of(
        environment: Environment<"files" | "process">,
        toolchain: () => Promise<Toolchain>,
    ): CompilerIdentity {
        return new CompilerIdentity(environment, toolchain);
    }

    /** Digest a file's bytes, null for a missing file. */
    async #file(path: string): Promise<string | null> {
        try {
            return await Digest.of(
                await this.environment.files.readFile(path),
                this.environment.crypto,
            );
        } catch (error) {
            if (FileSystemError.is(error, "NOT_FOUND")) {
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

        // identify Bun, the conversion's modules, the manifest writer's closure and the releases they run
        const compiler = await ModuleClosure.read(fileURLToPath(COMPILER_ENTRY), this.environment);
        const modules = await Promise.all(
            CONVERSION_MODULES.map((module) => this.#file(fileURLToPath(module))),
        );
        const writer = await ModuleClosure.read(fileURLToPath(MANIFEST_WRITER), this.environment);

        return await Digest.json(
            {
                layout: KEY_LAYOUT,
                bun: this.environment.process.runtime.revision,
                modules,
                manifest: await writer.digest(this.environment.crypto),
                releases: await compiler
                    .releases(CONVERSION_RELEASES)
                    .digest(this.environment.crypto),
            },
            this.environment.crypto,
        );
    }

    /** Identify the compiler by runtime, bundle and toolchain, else Bun, TypeScript and modules. */
    async #identifyCompiler(): Promise<Digest> {
        // identify a release's compiler by its runtime's build, its program's bundle and its toolchain's manifest
        const { runtime, script } = this.environment.process;
        if (runtime.isStandalone) {
            const { digest } = await this.toolchain();
            if (digest === undefined) {
                throw BuildError.of(
                    "INVARIANT_VIOLATED",
                    "a release's compiler runs without a toolchain",
                );
            }

            return await Digest.json(
                {
                    layout: KEY_LAYOUT,
                    bun: runtime.revision,
                    program: await this.#file(script),
                    toolchain: digest,
                },
                this.environment.crypto,
            );
        }

        // identify Bun, TypeScript and the compiler's modules that shape output, with every release
        const entry = fileURLToPath(COMPILER_ENTRY);
        const compiler = (await ModuleClosure.read(entry, this.environment)).select(
            COMPILER_SOURCES,
        );

        return await Digest.json(
            {
                layout: KEY_LAYOUT,
                bun: this.environment.process.runtime.revision,
                typescript: version,
                compiler: await compiler.digest(this.environment.crypto),
            },
            this.environment.crypto,
        );
    }
}
