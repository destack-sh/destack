import { readdir, readFile } from "node:fs/promises";
import { join, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { version } from "typescript";
import { type DependencyResolution, PackageJson } from "@destack/package";
import { PackageLocator } from "@destack/package/locator";
import { Digest, found, schema } from "@destack/schema";
import { BunProcess } from "@destack/check/bun";
import { BuildError, isMissing } from "../error/index.ts";
import { ModuleClosure } from "./closure.ts";
import type { ProgramImports } from "./scan.ts";
import { relativePath } from "../source/dependency.ts";
import { readDirectory } from "../build/directory.ts";

/** The entry a compiler process starts from, whose module closure identifies the toolchain. */
const COMPILER_ENTRY = new URL("../main.ts", import.meta.url);

/** The layout of cache keys, changed whenever keys derive from different inputs. */
const KEY_LAYOUT = 2;

/** A package.json as package identities read it. */
const Manifest = schema.looseObject({
    name: schema.string().min(1),
    version: schema.string().min(1),
    dependencies: schema.record(schema.string(), schema.string()).exactOptional(),
    peerDependencies: schema.record(schema.string(), schema.string()).exactOptional(),
    optionalDependencies: schema.record(schema.string(), schema.string()).exactOptional(),
    devDependencies: schema.record(schema.string(), schema.string()).exactOptional(),
    imports: schema.record(schema.string(), schema.unknown()).exactOptional(),
});

/** The identities of packages, files and directories a key derivation reads, each once. */
export class Identities {
    /** The installed packages and source packages the derivation found. */
    readonly #locator = new PackageLocator();
    /** Each package's identity and the packages it depends on, by directory. */
    readonly #packages = new Map<string, Promise<PackageIdentity>>();
    /** The manifest digests of the Destack dependencies' builds, by package name. */
    readonly #builds: ReadonlyMap<string, string>;

    /** Identify packages, Destack dependencies by their builds' manifests. */
    private constructor(builds: ReadonlyMap<string, string>) {
        this.#builds = builds;
    }

    /** Identify packages with a build's dependencies, Destack ones by their manifests. */
    static of(dependencies: Readonly<Record<string, DependencyResolution>>): Identities {
        return new Identities(
            new Map(
                Object.entries(dependencies).flatMap(([name, dependency]) =>
                    dependency.kind === "destack" ? [[name, dependency.manifest]] : [],
                ),
            ),
        );
    }

    /** Identify a package with its dependencies: releases by version, source packages by files. */
    async package(directory: string): Promise<Digest> {
        // collect the package and every package it depends on
        const visited = new Map<string, PackageIdentity>();
        const pending = [directory];
        for (const current of pending) {
            if (visited.has(current)) {
                continue;
            }
            const identity = await this.#identity(current);
            visited.set(current, identity);
            pending.push(...identity.dependencies);
        }

        return await Digest.json(
            [...visited.values()].map((identity) => identity.identity).toSorted(),
        );
    }

    /** List the directories of a package's dependencies, development ones included. */
    async installed(directory: string): Promise<readonly string[]> {
        // read the manifest's dependencies of every kind
        const manifest = await readManifest(directory);
        const names = new Set([
            ...PackageJson.runtimeDependencies(manifest),
            ...Object.keys(manifest.devDependencies ?? {}),
        ]);

        return [...names].flatMap((name) => this.#locator.directory(name, directory) ?? []);
    }

    /** Read a package's subpath imports, `#name` or `#prefix/*`, with all conditions' files. */
    async subpaths(directory: string): Promise<ReadonlyMap<string, readonly string[]>> {
        const manifest = await readManifest(directory);

        return new Map(
            Object.entries(manifest.imports ?? {}).map(([name, mapped]) => [
                name,
                subpathTargets(mapped).map((target) => join(directory, target)),
            ]),
        );
    }

    /** Digest a file's bytes, null for a missing file. */
    async file(path: string): Promise<string | null> {
        try {
            return await Digest.of(new Uint8Array(await readFile(path)));
        } catch (error) {
            if (isMissing(error)) {
                return null;
            }
            throw error;
        }
    }

    /** Digest a directory's files by their paths and bytes, null for a missing directory. */
    async directory(path: string): Promise<string | null> {
        try {
            const files = await readDirectory(path);

            return await Digest.json(
                await Promise.all(
                    [...files].map(async ([name, bytes]) => [name, await Digest.of(bytes)]),
                ),
            );
        } catch (error) {
            if (isMissing(error)) {
                return null;
            }
            throw error;
        }
    }

    /** Read a package's identity and the directories of the dependencies installed for it. */
    #identity(directory: string): Promise<PackageIdentity> {
        let identity = this.#packages.get(directory);
        if (identity === undefined) {
            identity = this.#read(directory);
            this.#packages.set(directory, identity);
        }

        return identity;
    }

    /** Identify a release by name and version, and a source package by manifests and sources. */
    async #read(directory: string): Promise<PackageIdentity> {
        // key a Destack dependency by its build's manifest, which records its dependencies
        const manifest = await readManifest(directory);
        const build = this.#builds.get(manifest.name);
        if (build !== undefined) {
            return { identity: `${manifest.name} ${build}`, dependencies: [] };
        }

        // locate each installed dependency, leaving absent optional ones
        const names = new Set(PackageJson.runtimeDependencies(manifest));
        const dependencies = [...names]
            .toSorted()
            .flatMap((name) => this.#locator.directory(name, directory) ?? []);

        // key an installed release by its version
        const release = `${manifest.name}@${manifest.version}`;
        if (directory.split(sep).includes("node_modules")) {
            return { identity: release, dependencies };
        }

        // key a source package by its manifests and the files of its sources
        const files = await Promise.all(
            ["package.json", "destack.json"].map(async (path) => [
                path,
                await this.file(join(directory, path)),
            ]),
        );
        for (const root of ["src", "locale"]) {
            for (const path of await listFiles(join(directory, root))) {
                files.push([`${root}/${path}`, await this.file(join(directory, root, path))]);
            }
        }

        return { identity: `${release} ${await Digest.json(files)}`, dependencies };
    }
}

/** A package's identity without its dependencies, and the directories of its dependencies. */
interface PackageIdentity {
    /** The release, and the digest of a source package's files. */
    readonly identity: string;
    /** The directories of the dependencies installed for the package. */
    readonly dependencies: readonly string[];
}

/** Identify the toolchain running this compiler. */
export async function identifyToolchain(identities: Identities): Promise<Digest> {
    // identify a standalone executable by its bytes
    if (BunProcess.isStandalone) {
        return await Digest.json({
            layout: KEY_LAYOUT,
            executable: await identities.file(process.execPath),
        });
    }

    // identify Bun, TypeScript and the modules the compiler's entry loads
    const compiler = await ModuleClosure.read(fileURLToPath(COMPILER_ENTRY));

    return await Digest.json({
        layout: KEY_LAYOUT,
        bun: BunProcess.revision(),
        typescript: version,
        compiler: await compiler.digest(),
    });
}

/** Key each module by toolchain, program, bytes and import cycle, and its read directories. */
export async function deriveModuleKeys(
    program: ProgramImports,
    root: string,
    toolchain: Digest,
    identities: Identities,
    compiled: ReadonlySet<string>,
): Promise<Map<string, Digest>> {
    // identify the program by its options and its packages
    const context = await Digest.json({
        toolchain,
        options: program.options,
        packages: await Promise.all(
            program.packages.map((directory) => identities.package(directory)),
        ),
    });

    // digest each import cycle after the cycles it imports, as cycles complete dependencies first
    const digests = new Map<string, Digest>();
    for (const cycle of new CycleFinder(program).cycles()) {
        const digest = await Digest.json(
            await describeCycle(cycle, program, root, digests, identities, compiled),
        );
        for (const file of cycle) {
            digests.set(file, digest);
        }
    }

    // key each module by the program, its path and its cycle's digest
    const keys = new Map<string, Digest>();
    for (const file of program.modules.keys()) {
        const path = relativePath(root, file);
        keys.set(file, await Digest.json({ context, path, cycle: found(digests, file) }));
    }

    return keys;
}

/** One module of an import cycle: its bytes, its imports and the directories it reads. */
interface CycleMember {
    /** The digest of the module's bytes. */
    readonly digest: Digest;
    /** Each import's target: a member path, a cycle digest, a file digest or the specifier. */
    readonly imports: Readonly<Record<string, string | null>>;
    /** The digest of each directory the module reads by path, null for a missing one. */
    readonly directories: Readonly<Record<string, string | null>>;
}

/** Describe each member of an import cycle, keying the cycles it imports by their digests. */
async function describeCycle(
    cycle: readonly string[],
    program: ProgramImports,
    root: string,
    digests: ReadonlyMap<string, Digest>,
    identities: Identities,
    compiled: ReadonlySet<string>,
): Promise<Record<string, CycleMember>> {
    // describe each member by its path
    const members = new Set(cycle);
    const described: Record<string, CycleMember> = {};
    for (const file of cycle) {
        // describe each import's target: a member path, a cycle digest, a file digest or the specifier
        const module = found(program.modules, file);
        const imports: Record<string, string | null> = {};
        for (const { specifier, target } of module.imports) {
            if (target.kind === "module" && members.has(target.file)) {
                imports[specifier] = relativePath(root, target.file);
            } else if (target.kind === "module") {
                imports[specifier] = found(digests, target.file);
            } else if (target.kind === "file") {
                imports[specifier] = await identities.file(target.file);
            } else {
                imports[specifier] = specifier;
            }
        }

        // digest the directories a compiled member reads, which its output ships
        const directories: Record<string, string | null> = {};
        for (const directory of compiled.has(file) ? module.directories : []) {
            directories[relativePath(root, directory)] = await identities.directory(directory);
        }
        described[relativePath(root, file)] = { digest: module.digest, imports, directories };
    }

    return described;
}

/** List the package's modules a program imports from its entries, the entries included. */
export function importClosure(program: ProgramImports, entries: readonly string[]): Set<string> {
    // require each entry among the package's modules
    for (const entry of entries) {
        if (!program.modules.has(entry)) {
            throw new BuildError("BUILD_FAILED", `the program lacks the entry ${entry}`);
        }
    }

    // follow the imports between the package's modules
    const visited = new Set<string>();
    const pending = [...entries];
    for (const file of pending) {
        if (visited.has(file)) {
            continue;
        }
        visited.add(file);
        for (const { target } of found(program.modules, file).imports) {
            if (target.kind === "module") {
                pending.push(target.file);
            }
        }
    }

    return visited;
}

/** A search for the import cycles of a program's modules, each after its imports (Tarjan). */
class CycleFinder {
    /** The program whose modules the finder visits. */
    readonly program: ProgramImports;
    /** The cycles found so far, each after the cycles it imports. */
    readonly #cycles: string[][] = [];
    /** The number of each visited module, in visiting order. */
    readonly #indices = new Map<string, number>();
    /** The lowest number each module links to on the stack. */
    readonly #lowest = new Map<string, number>();
    /** The visited modules of the open cycles. */
    readonly #stack: string[] = [];
    /** The modules on the stack. */
    readonly #stacked = new Set<string>();

    /** Find the cycles of a program's modules. */
    constructor(program: ProgramImports) {
        this.program = program;
    }

    /** List every cycle, visiting modules in path order. */
    cycles(): string[][] {
        for (const file of [...this.program.modules.keys()].toSorted()) {
            if (!this.#indices.has(file)) {
                this.#visit(file);
            }
        }

        return this.#cycles;
    }

    /** Visit a module and the modules it imports, closing its cycle at its first module. */
    #visit(file: string): void {
        // number the module and stack it
        this.#indices.set(file, this.#indices.size);
        this.#lowest.set(file, found(this.#indices, file));
        this.#stack.push(file);
        this.#stacked.add(file);

        // visit each imported module, lowering the module's number to the import's
        for (const { target } of found(this.program.modules, file).imports) {
            if (target.kind !== "module") {
                continue;
            }
            if (!this.#indices.has(target.file)) {
                this.#visit(target.file);
                this.#lower(file, found(this.#lowest, target.file));
            } else if (this.#stacked.has(target.file)) {
                this.#lower(file, found(this.#indices, target.file));
            }
        }

        // close a cycle at its first module
        if (found(this.#lowest, file) === found(this.#indices, file)) {
            this.#cycles.push(this.#close(file));
        }
    }

    /** Lower a module's number to another, when lower. */
    #lower(file: string, number: number): void {
        this.#lowest.set(file, Math.min(found(this.#lowest, file), number));
    }

    /** Pop the stack down to a cycle's first module, returning the cycle's modules. */
    #close(file: string): string[] {
        // pop until the cycle's first module
        const cycle: string[] = [];
        let member: string | undefined;
        do {
            member = this.#stack.pop();
            if (member === undefined) {
                throw new BuildError("BUILD_FAILED", "an import cycle lost its modules");
            }
            this.#stacked.delete(member);
            cycle.push(member);
        } while (member !== file);

        return cycle;
    }
}

/** List the regular files below a directory by relative path, none for an absent directory. */
async function listFiles(directory: string): Promise<string[]> {
    try {
        const entries = await readdir(directory, { recursive: true, withFileTypes: true });

        return entries
            .filter((entry) => entry.isFile())
            .map((entry) => relativePath(directory, join(entry.parentPath, entry.name)))
            .toSorted();
    } catch (error) {
        if (isMissing(error)) {
            return [];
        }
        throw error;
    }
}

/** Read a package's manifest. */
async function readManifest(directory: string): Promise<schema.Infer<typeof Manifest>> {
    return Manifest.parse(JSON.parse(await readFile(join(directory, "package.json"), "utf8")));
}

/** List the files a subpath import maps to under any condition, none for an excluded one. */
function subpathTargets(mapped: unknown): string[] {
    // take a path, or every condition's paths
    if (typeof mapped === "string") {
        return [mapped];
    }
    // read each condition of a conditions object
    else if (typeof mapped === "object" && mapped !== null) {
        return Object.values(mapped).flatMap((value) => subpathTargets(value));
    }
    // leave out an excluded subpath, which maps to null
    else {
        return [];
    }
}
