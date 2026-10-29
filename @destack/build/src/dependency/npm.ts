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
import { NATIVE_ADDON_RUNTIMES, Runtime } from "@destack/package/runtime";
import { NpmConversion } from "../compile/conversion.ts";
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

/** An npm dependency taken by its unit's build. */
export type NpmDependency = Extract<DependencyRelease, { readonly kind: "npm" }>;

/** The npm packages a lockfile locks, each opened and stored once as a unit. */
export class NpmInstallation {
    /** The npm releases the lockfile locks, by `name@version`. */
    readonly locked: Readonly<Record<string, NpmRelease>>;
    /** The store of the units' builds. */
    readonly #store: PackageStore;
    /** The identity of the npm conversion building the units, which their keys include. */
    readonly #conversion: Digest;
    /** The files the installed copies are read from, and the crypto keying their builds. */
    readonly #environment: Environment<"files">;
    /** The units opened so far, by their copy in Bun's store. */
    readonly #opened = new Map<string, NpmUnit>();
    /** The last open, after which the next one runs, so concurrent units open each copy once. */
    #opening: Promise<unknown> = Promise.resolve();

    /** Store the locked releases, and the store and conversion the units build with. */
    constructor(
        locked: Readonly<Record<string, NpmRelease>>,
        store: PackageStore,
        conversion: Digest,
        environment: Environment<"files">,
    ) {
        // store the locked releases and what the units build with
        this.locked = locked;
        this.#store = store;
        this.#conversion = conversion;
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
        const unit = new NpmUnit(release, directory, dependencies, manifest, this.#environment);
        this.#opened.set(directory, unit);

        return unit;
    }

    /** Store a unit's build for a runtime once, keyed by conversion, release and closure. */
    async store(unit: NpmUnit, runtime: Runtime): Promise<NpmDependency> {
        // key the build by what decides its bytes: the conversion, the release, the runtime and the locked closure
        const { package: released, integrity } = unit.release;
        const key = await Digest.json(
            {
                kind: "npm",
                conversion: this.#conversion,
                package: { ...released, integrity },
                runtime,
                closure: unit
                    .closure()
                    .map((each) => `${each.release.package.name}@${each.release.package.version}`)
                    .toSorted(),
            },
            this.#environment.crypto,
        );

        // take the stored build, else build it on the builds of the dependencies its modules import
        const manifest = await this.#store.cachedBuild(key, async () => {
            const built = await unit.build(runtime, (each) => this.store(each, runtime));

            return await this.#store.put(built);
        });

        return { kind: "npm", ...unit.release, manifest };
    }
}

/** An npm package installed in the workspace's Bun store, built as one unit per runtime. */
export class NpmUnit {
    /** The release the lockfile locks. */
    readonly release: NpmRelease;
    /** The installed copy in Bun's store. */
    readonly directory: string;
    /** The installed packages it requires, peers excepted, by the names it imports them by. */
    readonly dependencies: ReadonlyMap<string, NpmUnit>;
    /** The dependencies, optional dependencies and peers its package.json declares. */
    readonly requirements: RuntimeRequirements;
    /** The files the installed copy is read from, and the crypto digesting its build. */
    readonly #environment: Environment<"files">;

    /** Store an installed package's release, copy, dependencies and requirements. */
    constructor(
        release: NpmRelease,
        directory: string,
        dependencies: ReadonlyMap<string, NpmUnit>,
        requirements: RuntimeRequirements,
        environment: Environment<"files">,
    ) {
        // store the release, its copy and its requirements
        this.release = release;
        this.directory = directory;
        this.dependencies = dependencies;
        this.requirements = requirements;
        this.#environment = environment;
    }

    /** List the peers it declares, which the tree it sits in provides. */
    get peers(): string[] {
        return PackageJson.peers(this.requirements);
    }

    /** List the peers it requires, which every package importing it provides beside it. */
    get requiredPeers(): string[] {
        return this.peers.filter((peer) => !PackageJson.isOptional(this.requirements, peer));
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

    /** Build the unit for a runtime in memory: Bun's as installed, the others converted to ESM. */
    async build(
        runtime: Runtime,
        store: (dependency: NpmUnit) => Promise<NpmDependency>,
    ): Promise<BuildReader> {
        // refuse a native addon on a runtime that cannot load it
        const files = await NpmUnit.files(this.directory, this.#environment.files);
        const isAddon = [...files.keys()].some((path) => path.endsWith(ADDON_EXTENSION));
        const { name } = this.release.package;
        if (isAddon && !NATIVE_ADDON_RUNTIMES.has(runtime)) {
            throw BuildError.of(
                "RUNTIME_MODULE_UNSUPPORTED",
                `${name} is a native addon, which ${runtime} cannot load`,
            );
        }

        // take the installed files for Bun, which runs them as installed, else convert them
        const converted =
            runtime === "bun"
                ? { files, exports: {}, imported: new Set(this.dependencies.keys()) }
                : await new NpmConversion(
                      name,
                      [...this.dependencies.keys()],
                      this.peers,
                      runtime,
                      this.#environment.files,
                  ).convert(files);

        // store the dependencies its modules import for the runtime, at once
        const imported = [...this.dependencies].filter(([each]) => converted.imported.has(each));
        const dependencies = Object.fromEntries(
            await Promise.all(
                imported.map(async ([each, unit]) => [each, await store(unit)] as const),
            ),
        );

        // write its files below the runtime, described by its exports and the dependencies it imports
        const written = new Map<string, Uint8Array<ArrayBuffer>>();
        const writer = new BuildWriter(
            { write: async (path, bytes) => void written.set(path, bytes) },
            this.#environment.crypto,
        );
        await Promise.all(
            [...converted.files].map(([path, bytes]) => writer.write(`${runtime}/${path}`, bytes)),
        );
        const output: PackageOutput = {
            runtime,
            emit: true,
            workloads: {},
            views: {},
            directory: runtime,
            exports: Object.fromEntries(
                Object.entries(converted.exports).map(([subpath, module]) => [
                    subpath,
                    { path: `${runtime}/${module}`, imports: [] },
                ]),
            ),
            dependencies,
        };
        const { integrity } = this.release;
        const manifest = await writer.finish({
            package: { ...this.release.package, integrity },
            outputs: { [runtime]: output },
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
