import { readFile } from "node:fs/promises";
import { isBuiltin } from "node:module";
import { dirname, extname, join, relative, sep } from "node:path";
import { Digest, schema } from "@destack/schema";
import { BuildError, isMissing } from "../error/index.ts";

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

    /** Keep the identities of a closure's modules. */
    private constructor(modules: ReadonlyMap<string, string>) {
        this.modules = modules;
    }

    /** Read the modules an entry imports, sources by their bytes and npm releases by name. */
    static async read(entry: string): Promise<ModuleClosure> {
        // follow each source's imports once, keying a module under node_modules by its release
        const modules = new Map<string, string>();
        const visited = new Set<string>();
        const pending = [entry];
        for (let file = pending.pop(); file !== undefined; file = pending.pop()) {
            if (visited.has(file)) {
                continue;
            }
            visited.add(file);
            const owner = await ModuleClosure.#package(file);
            const path = relative(owner.directory, file).split(sep).join("/");
            // identify an npm release by its name and version, without reading its modules
            if (file.split(sep).includes("node_modules")) {
                modules.set(`${owner.name}@${owner.version}`, "release");
            }
            // identify a source by its package, path and bytes, and follow its imports
            else {
                const bytes = new Uint8Array(await readFile(file));
                modules.set(`${owner.name}/${path}`, await Digest.of(bytes));
                pending.push(...ModuleClosure.#imports(new TextDecoder().decode(bytes), file));
            }
        }

        return new ModuleClosure(
            new Map([...modules].toSorted(([left], [right]) => (left < right ? -1 : 1))),
        );
    }

    /** Digest the closure by its modules' identities. */
    async digest(): Promise<Digest> {
        return await Digest.json([...this.modules]);
    }

    /** Resolve the imports Bun loads of a module to files, leaving runtime modules out. */
    static #imports(text: string, file: string): string[] {
        // scan the module past its hashbang as Bun's transpiler loads it
        const loader = LOADERS[extname(file)];
        if (loader === undefined) {
            throw new BuildError(
                "BUILD_FAILED",
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
    ): Promise<{ readonly directory: string; readonly name: string; readonly version: string }> {
        // take the nearest manifest declaring a package
        for (
            let directory = dirname(file);
            dirname(directory) !== directory;
            directory = dirname(directory)
        ) {
            const text = await readFile(join(directory, "package.json"), "utf8").catch(
                (error: unknown) => {
                    if (isMissing(error)) {
                        return undefined;
                    }
                    throw error;
                },
            );
            const parsed =
                text === undefined ? undefined : ModulePackage.safeParse(JSON.parse(text));
            if (parsed?.success === true) {
                return { directory, ...parsed.data };
            }
        }
        throw new BuildError("BUILD_FAILED", `no package declares the module ${file}`);
    }
}
