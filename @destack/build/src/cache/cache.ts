import { fileURLToPath } from "node:url";
import { version } from "typescript";
import { type Environment, FileSystemError } from "@destack/environment";
import { Digest } from "@destack/schema";
import { ModuleClosure } from "./closure.ts";

/** The entry a compiler process starts from, whose module closure identifies the toolchain. */
const COMPILER_ENTRY = new URL("../main.ts", import.meta.url);

/** The layout of cache keys, changed whenever keys derive from different inputs. */
const KEY_LAYOUT = 4;

/** The workspace sources that shape build output and keys: the compiler, digests and catalogs. */
const TOOLCHAIN_SOURCES: ReadonlySet<string> = new Set([
    "@destack/build/",
    "@destack/package/",
    "@destack/schema/src/digest/",
    "@destack/schema/src/json/",
    "@destack/locale/src/message/catalog.ts",
]);

/** The modules converting npm packages, whose bytes alone shape an npm unit's build. */
const CONVERSION_MODULES = [
    new URL("../compile/conversion.ts", import.meta.url),
    new URL("../dependency/npm.ts", import.meta.url),
    new URL("../../../package/src/manifest/writer.ts", import.meta.url),
];

/** The npm releases converting npm packages: the bundler and the CommonJS lexer. */
const CONVERSION_RELEASES: ReadonlySet<string> = new Set(["rolldown", "cjs-module-lexer"]);

/** The toolchain this process loaded: its compiler and npm conversion, which build keys read. */
export class Toolchain {
    /** The files and process the toolchain is read through. */
    readonly environment: Environment<"files" | "process">;
    /** The digest of the toolchain, read once. */
    #digest: Promise<Digest> | undefined;
    /** The digest of the npm conversion, read once. */
    #conversion: Promise<Digest> | undefined;

    /** Read the toolchain through an environment. */
    private constructor(environment: Environment<"files" | "process">) {
        this.environment = environment;
    }

    /** Read the toolchain through an environment. */
    static of(environment: Environment<"files" | "process">): Toolchain {
        return new Toolchain(environment);
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

    /** Identify the toolchain running this compiler, once, as its code stays loaded. */
    digest(): Promise<Digest> {
        this.#digest ??= this.#identifyToolchain();

        return this.#digest;
    }

    /** Identify the npm conversion, once, as its code stays loaded. */
    conversion(): Promise<Digest> {
        this.#conversion ??= this.#identifyConversion();

        return this.#conversion;
    }

    /** Identify the npm conversion by its executable, else Bun, its modules and their releases. */
    async #identifyConversion(): Promise<Digest> {
        // identify a standalone executable by its bytes, as its toolchain
        if (this.environment.process.runtime.isStandalone) {
            return await this.digest();
        }

        // identify Bun, the conversion's modules and the releases they run
        const compiler = await ModuleClosure.read(fileURLToPath(COMPILER_ENTRY), this.environment);
        const modules = await Promise.all(
            CONVERSION_MODULES.map((module) => this.#file(fileURLToPath(module))),
        );

        return await Digest.json(
            {
                layout: KEY_LAYOUT,
                bun: this.environment.process.runtime.revision,
                modules,
                releases: await compiler
                    .releases(CONVERSION_RELEASES)
                    .digest(this.environment.crypto),
            },
            this.environment.crypto,
        );
    }

    /** Identify the toolchain by its executable, else Bun, TypeScript and the compiler. */
    async #identifyToolchain(): Promise<Digest> {
        // identify a standalone executable by its bytes
        if (this.environment.process.runtime.isStandalone) {
            return await Digest.json(
                {
                    layout: KEY_LAYOUT,
                    executable: await this.#file(this.environment.process.executable),
                },
                this.environment.crypto,
            );
        }

        // identify Bun, TypeScript and the compiler's modules that shape output, with every release
        const entry = fileURLToPath(COMPILER_ENTRY);
        const compiler = (await ModuleClosure.read(entry, this.environment)).select(
            TOOLCHAIN_SOURCES,
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
