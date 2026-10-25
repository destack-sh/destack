import { join } from "node:path";
import type { Environment, FileSystem } from "@destack/environment";
import {
    type DependencyRelease,
    type NpmRelease,
    PackageJson,
    RuntimeRequirements,
} from "@destack/package";
import { PackageLocator } from "@destack/package/locator";
import { BuildReader, BuildWriter, type PackageOutput } from "@destack/package/manifest";
import { NATIVE_ADDON_RUNTIMES, type Runtime, SERVER_RUNTIMES } from "@destack/package/runtime";
import { Digest, found, nth, schema } from "@destack/schema";
import { BuildError } from "../error/index.ts";
import type { PackageStore } from "../store/index.ts";

/** The files read at once, 64: well under macOS's default limit of 256 open files. */
const CONCURRENT_READS = 64;

/** The extension of a native addon, which Bun loads and workerd cannot. */
const ADDON_EXTENSION = ".node";

/** The package.json fields an npm unit reads: its release and its requirements. */
const Manifest = RuntimeRequirements.extend({
    name: schema.string().min(1),
    version: schema.string().min(1),
});

/** The workerd conversion of one installed npm package: its copy, name, dependencies and peers. */
export interface NpmConversionRequest {
    /** The installed copy in Bun's store. */
    readonly source: string;
    /** The package's name. */
    readonly name: string;
    /** The installed packages it requires. */
    readonly dependencies: readonly string[];
    /** The peers it declares. */
    readonly peers: readonly string[];
}

/** Convert an installed npm package for workerd, answering its files and what it imports. */
export type NpmConverter = (request: NpmConversionRequest) => Promise<{
    readonly files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>;
    readonly imported: ReadonlySet<string>;
}>;

/** An npm dependency taken by its unit's build. */
export type NpmDependency = Extract<DependencyRelease, { readonly kind: "npm" }>;

/** The npm packages a lockfile locks, each opened and stored once as a unit. */
export class NpmInstallation {
    /** The npm releases the lockfile locks, by `name@version`. */
    readonly locked: Readonly<Record<string, NpmRelease>>;
    /** The store of the units' builds. */
    readonly #store: PackageStore;
    /** The identity of the toolchain building the units, which their keys include. */
    readonly #toolchain: Digest;
    /** The files the installed copies are read from, and the crypto keying their builds. */
    readonly #environment: Environment<"files">;
    /** The units opened so far, by their copy in Bun's store. */
    readonly #opened = new Map<string, NpmUnit>();
    /** The units' stored builds so far, by their copy in Bun's store. */
    readonly #stored = new Map<string, Promise<NpmDependency>>();
    /** The last open, after which the next one runs, so concurrent units open each copy once. */
    #opening: Promise<unknown> = Promise.resolve();

    /** Store the locked releases, and the store and toolchain the units build with. */
    constructor(
        locked: Readonly<Record<string, NpmRelease>>,
        store: PackageStore,
        toolchain: Digest,
        environment: Environment<"files">,
    ) {
        // store the locked releases and what the units build with
        this.locked = locked;
        this.#store = store;
        this.#toolchain = toolchain;
        this.#environment = environment;
    }

    /** Open the npm package installed in a directory and every package it requires, once each. */
    open(directory: string): Promise<NpmUnit> {
        const opened = this.#opening.then(() => this.#open(directory, new Set()));
        this.#opening = opened.catch(() => undefined);

        return opened;
    }

    /** Open an installed package and the packages it requires, refusing a cycle through them. */
    async #open(directory: string, visiting: ReadonlySet<string>): Promise<NpmUnit> {
        // take an opened unit, refusing a cycle through one being opened
        const known = this.#opened.get(directory);
        if (known !== undefined) {
            return known;
        }
        if (visiting.has(directory)) {
            throw BuildError.of("DEPENDENCY_INVALID", `npm dependency cycle through ${directory}`);
        }

        // lock the release by its installed name and version
        const manifest = Manifest.parse(
            JSON.parse(await this.#environment.files.readTextFile(join(directory, "package.json"))),
        );
        const key = `${manifest.name}@${manifest.version}`;
        const release = this.locked[key];
        if (release === undefined) {
            throw BuildError.of("LOCKFILE_INVALID", `the lockfile has no ${key}`);
        }

        // open each installed package it requires, leaving its peers to the tree it sits in
        const locator = new PackageLocator(this.#environment.files);
        const dependencies = new Map<string, NpmUnit>();
        const required = Object.keys({
            ...manifest.dependencies,
            ...manifest.optionalDependencies,
        });
        const peers = new Set(PackageJson.peers(manifest));
        const next = new Set([...visiting, directory]);
        for (const name of required.toSorted()) {
            const installed = await locator.directory(name, directory);
            // leave out an absent optional dependency
            if (installed === undefined && manifest.optionalDependencies?.[name] !== undefined) {
                continue;
            }
            // refuse another absent one
            else if (installed === undefined) {
                throw BuildError.of(
                    "DEPENDENCY_UNRESOLVED",
                    `missing dependency of ${key}: ${name}`,
                );
            }
            // open the installed one
            else {
                const real = await this.#environment.files.realPath(installed);
                dependencies.set(name, await this.#open(real, next));
            }
        }
        const unit = new NpmUnit(release, directory, dependencies, peers, this.#environment);
        this.#opened.set(directory, unit);

        return unit;
    }

    /** Store a unit's build for every server runtime on its dependencies' builds, once. */
    async store(unit: NpmUnit, convert: NpmConverter): Promise<NpmDependency> {
        // store each unit once, whichever unit requires it
        let dependency = this.#stored.get(unit.directory);
        if (dependency === undefined) {
            dependency = this.#storeOnce(unit, convert);
            this.#stored.set(unit.directory, dependency);
        }

        return await dependency;
    }

    /** Store a unit's build after its dependencies', keyed by their builds and the release. */
    async #storeOnce(unit: NpmUnit, convert: NpmConverter): Promise<NpmDependency> {
        // store its dependencies first, which its key and build record
        const dependencies: Record<string, NpmDependency> = {};
        for (const [name, each] of unit.dependencies) {
            dependencies[name] = await this.store(each, convert);
        }

        // key the build by the toolchain, the release and the dependencies' builds
        const { package: released, integrity } = unit.release;
        const key = await Digest.json(
            {
                kind: "npm",
                toolchain: this.#toolchain,
                package: { ...released, integrity },
                dependencies: Object.fromEntries(
                    Object.entries(dependencies).map(([name, each]) => [name, each.manifest]),
                ),
            },
            this.#environment.crypto,
        );

        // take the stored build, building and storing it once when missing
        const manifest = await this.#store.cachedBuild(key, async () => {
            // read the runtimes each dependency's build loads on, then build on them
            const loadable = new Map<string, ReadonlySet<Runtime>>();
            for (const [name, each] of Object.entries(dependencies)) {
                const { outputs } = await this.#store.get(each.manifest);
                loadable.set(name, new Set(Object.values(outputs).map((output) => output.runtime)));
            }

            return await this.#store.put(await unit.build(dependencies, loadable, convert));
        });

        return { kind: "npm", ...unit.release, manifest };
    }
}

/** An npm package installed in the workspace's Bun store, built as one unit for server runtimes. */
export class NpmUnit {
    /** The release the lockfile locks. */
    readonly release: NpmRelease;
    /** The installed copy in Bun's store. */
    readonly directory: string;
    /** The installed packages it requires, peers excepted, by the names it imports them by. */
    readonly dependencies: ReadonlyMap<string, NpmUnit>;
    /** The peers it declares, which the tree it sits in provides. */
    readonly peers: ReadonlySet<string>;
    /** The files the installed copy is read from, and the crypto digesting its build. */
    readonly #environment: Environment<"files">;

    /** Store an installed package's release, copy, dependencies and peers. */
    constructor(
        release: NpmRelease,
        directory: string,
        dependencies: ReadonlyMap<string, NpmUnit>,
        peers: ReadonlySet<string>,
        environment: Environment<"files">,
    ) {
        // store the release, its copy and its requirements
        this.release = release;
        this.directory = directory;
        this.dependencies = dependencies;
        this.peers = peers;
        this.#environment = environment;
    }

    /** List this unit and every unit it requires, each once. */
    closure(): NpmUnit[] {
        // walk the dependencies once each
        const units = new Set<NpmUnit>();
        const pending: NpmUnit[] = [this];
        for (let unit = pending.pop(); unit !== undefined; unit = pending.pop()) {
            if (!units.has(unit)) {
                units.add(unit);
                pending.push(...unit.dependencies.values());
            }
        }

        return [...units];
    }

    /** Build the unit in memory: Bun's output as installed, workerd's converted to ESM. */
    async build(
        dependencies: Readonly<Record<string, NpmDependency>>,
        loadable: ReadonlyMap<string, ReadonlySet<Runtime>>,
        convert: NpmConverter,
    ): Promise<BuildReader> {
        // read the installed copy's files once, leaving workerd out for a native addon
        const files = await NpmUnit.files(this.directory, this.#environment.files);
        const written = new Map<string, Uint8Array<ArrayBuffer>>();
        const writer = new BuildWriter(
            {
                write: async (path, bytes) => {
                    written.set(path, bytes);
                },
            },
            this.#environment.crypto,
        );
        const isAddon = [...files.keys()].some((path) => path.endsWith(ADDON_EXTENSION));
        const runtimes = SERVER_RUNTIMES.filter(
            (runtime) => !isAddon || NATIVE_ADDON_RUNTIMES.has(runtime),
        );

        // copy the package for Bun, which runs it as installed, and convert it for workerd, together
        const built = await Promise.all(
            runtimes.map(async (runtime) => {
                // take the installed files for Bun and the converted ones for workerd, writing each
                const converted =
                    runtime === "workerd"
                        ? await convert({
                              source: this.directory,
                              name: this.release.package.name,
                              dependencies: [...this.dependencies.keys()],
                              peers: [...this.peers],
                          })
                        : { files, imported: new Set(Object.keys(dependencies)) };

                // leave the runtime out when a dependency it imports has no output for it
                const imported = Object.entries(dependencies).filter(([name]) =>
                    converted.imported.has(name),
                );
                if (!imported.every(([name]) => found(loadable, name).has(runtime))) {
                    return [];
                }

                // write its files and describe it by the dependencies it imports
                await Promise.all(
                    [...converted.files].map(([path, bytes]) =>
                        writer.write(`${runtime}/${path}`, bytes),
                    ),
                );
                const output: PackageOutput = {
                    runtime,
                    emit: true,
                    workloads: {},
                    views: {},
                    directory: runtime,
                    exports: {},
                    dependencies: Object.fromEntries(imported),
                };

                return [[runtime, output] as const];
            }),
        );
        const outputs = Object.fromEntries(built.flat());

        // finish the build under the release's name, version and integrity
        const { package: released, integrity } = this.release;
        const manifest = await writer.finish({
            package: { ...released, integrity },
            outputs,
            dependencies,
        });

        return new BuildReader(
            manifest,
            async (path) => new Blob([found(written, path)]).stream(),
            this.#environment.crypto,
        );
    }

    /** Read every file of an installed copy by its path, leaving the copy's node_modules out. */
    static async files(
        directory: string,
        files: FileSystem,
    ): Promise<Map<string, Uint8Array<ArrayBuffer>>> {
        // list each file, links followed, in path order
        const entries = await files.readDirectory(directory, { recursive: true });
        const listed = entries
            .filter((entry) => entry.kind === "file" || entry.kind === "symlink")
            .map((entry) => ({ path: entry.name, isLink: entry.kind === "symlink" }))
            .filter(({ path }) => !path.split("/").includes("node_modules"));
        const paths = listed.map(({ path }) => path).toSorted();

        // refuse a link to a folder, whose files a recursive listing never lists
        const links = listed.filter((entry) => entry.isLink).map((entry) => entry.path);
        const linked = await Promise.all(links.map((path) => files.stat(join(directory, path))));
        const folder = links.find((_path, index) => nth(linked, index).kind === "directory");
        if (folder !== undefined) {
            throw BuildError.of("DEPENDENCY_INVALID", `npm package links a folder: ${folder}`);
        }

        // read the files a bounded number at a time
        const contents = new Map<string, Uint8Array<ArrayBuffer>>();
        for (let start = 0; start < paths.length; start += CONCURRENT_READS) {
            const batch = paths.slice(start, start + CONCURRENT_READS);
            const read = await Promise.all(
                batch.map((path) => files.readFile(join(directory, path))),
            );
            for (const [index, path] of batch.entries()) {
                contents.set(path, new Uint8Array(nth(read, index)));
            }
        }

        return contents;
    }
}
