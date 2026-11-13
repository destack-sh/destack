import { join } from "node:path";
import { type Environment, type FileSystem, Once } from "@destack/environment";
import {
    type DependencyRelease,
    type NpmRelease,
    PackageJson,
    RuntimeRequirements,
} from "@destack/package";
import { PackageFile } from "@destack/package/file";
import { PackageLocator } from "@destack/package/locator";
import { BuildReader, BuildWriter, type OutputContent } from "@destack/package/manifest";
import { HOST_RUNTIME, Runtime } from "@destack/package/runtime";
import { NpmConversion } from "../compile/conversion.ts";
import { Digest, found, nth, schema } from "@destack/schema";
import { BuildError } from "../error/index.ts";
import type { BuildStore } from "../store/index.ts";

/** The Destack version this toolchain builds with: its own release's. */
const DESTACK_VERSION = import.meta.destack.package.version;

/** The files read at once, 64: well under macOS's default limit of 256 open files. */
const CONCURRENT_READS = 64;

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
    readonly #store: BuildStore;
    /** The identity of the conversion building the units, which keys their builds. */
    readonly conversion: Digest;
    /** The files the installed copies are read from, and the crypto keying their builds. */
    readonly #environment: Environment<"files">;
    /** The units opened so far, by their copy in Bun's store. */
    readonly #opened = new Map<string, NpmUnit>();
    /** The last open, after which the next one runs, so concurrent units open each copy once. */
    #opening: Promise<unknown> = Promise.resolve();

    /** Store the locked releases, and the store and conversion the units build with. */
    constructor(
        locked: Readonly<Record<string, NpmRelease>>,
        store: BuildStore,
        conversion: Digest,
        environment: Environment<"files">,
    ) {
        // store the locked releases and what the units build with
        this.locked = locked;
        this.#store = store;
        this.conversion = conversion;
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
            throw BuildError.of(
                "DEPENDENCY_INVALID",
                `npm dependency '${directory}' is invalid: it is part of a dependency cycle`,
            );
        }

        // lock the release by its installed name and version
        const manifest = Manifest.parse(
            JSON.parse(await this.#environment.files.readTextFile(join(directory, "package.json"))),
        );
        const key = `${manifest.name}@${manifest.version}`;
        const release = this.locked[key];
        if (release === undefined) {
            throw BuildError.of("LOCKFILE_INVALID", `lockfile is invalid: it has no '${key}'`);
        }

        // open each installed package it requires
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
                    "DEPENDENCY_INVALID",
                    `dependency '${name}' of '${key}' is invalid: it is missing`,
                );
            }
            // open the installed one
            else {
                const real = await this.#environment.files.realPath(installed);
                dependencies.set(name, await this.#open(real, next));
            }
        }

        // open each required peer the install linked beside it, unless the peer leads back to it
        const requiredPeers = PackageJson.peers(manifest).filter(
            (peer) => !PackageJson.isOptional(manifest, peer),
        );
        for (const peer of requiredPeers.toSorted()) {
            const installed = await locator.directory(peer, directory);
            const real =
                installed === undefined
                    ? undefined
                    : await this.#environment.files.realPath(installed);
            const isLinked = real !== undefined && !dependencies.has(peer);
            if (isLinked && !(await this.#reaches(real, directory))) {
                dependencies.set(peer, await this.#open(real, next));
            }
        }
        const unit = new NpmUnit(release, directory, dependencies, manifest, this.#environment);
        this.#opened.set(directory, unit);

        return unit;
    }

    /** Report whether an installed package leads to another through its requirements or peers. */
    async #reaches(from: string, target: string): Promise<boolean> {
        // walk the installed packages each one requires, each once
        const locator = new PackageLocator(this.#environment.files);
        const seen = new Set<string>();
        const pending = [from];
        for (let next = pending.pop(); next !== undefined; next = pending.pop()) {
            // stop at the target, else read the next copy's requirements and peers once
            if (next === target) {
                return true;
            } else if (!seen.has(next)) {
                seen.add(next);
                const manifest = Manifest.parse(
                    JSON.parse(
                        await this.#environment.files.readTextFile(join(next, "package.json")),
                    ),
                );
                const required = [
                    ...Object.keys({ ...manifest.dependencies, ...manifest.optionalDependencies }),
                    ...PackageJson.peers(manifest),
                ];
                for (const name of required) {
                    const installed = await locator.directory(name, next);
                    if (installed !== undefined) {
                        pending.push(await this.#environment.files.realPath(installed));
                    }
                }
            }
        }

        return false;
    }

    /** Store a unit's build once, an output per runtime, keyed by release and closure. */
    async store(unit: NpmUnit): Promise<NpmDependency> {
        // key the build by what decides its bytes
        const { package: released, integrity } = unit.release;
        const key = await Digest.json(
            {
                kind: "npm",
                conversion: this.conversion,
                package: { ...released, integrity },
                closure: unit
                    .closure()
                    .map((each) => `${each.release.package.name}@${each.release.package.version}`)
                    .toSorted(),
            },
            this.#environment.crypto,
        );

        // take the stored build, else build it on the builds of the dependencies its modules import
        const { manifest } = await this.#store.cachedBuild(key, async () => {
            const built = await unit.build((each) => this.store(each));

            return await this.#store.put(built);
        });

        return { kind: "npm", package: { ...released, integrity }, manifest };
    }
}

/** An npm package in the workspace's Bun store, built as one unit with an output per runtime. */
export class NpmUnit {
    /** The release the lockfile locks. */
    readonly release: NpmRelease;
    /** The installed copy in Bun's store. */
    readonly directory: string;
    /** The installed packages it requires and the peers its install linked, by import name. */
    readonly dependencies: ReadonlyMap<string, NpmUnit>;
    /** The dependencies, optional dependencies and peers its package.json declares. */
    readonly requirements: RuntimeRequirements;
    /** The files the installed copy is read from, and the crypto digesting its build. */
    readonly #environment: Environment<"files">;
    /** The runtimes the unit runs on, read once. */
    readonly #runtimes = new Once<readonly Runtime[]>();

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

    /** List the dependencies it imports, and those it installs as their required peers. */
    #provided(imported: ReadonlySet<string>): [string, NpmUnit][] {
        // take the imported ones, then each required peer of theirs that this unit installs
        const included = new Set(
            [...this.dependencies.keys()].filter((each) => imported.has(each)),
        );
        for (const name of included) {
            const unit = this.dependencies.get(name);
            for (const peer of unit?.requiredPeers ?? []) {
                if (this.dependencies.has(peer)) {
                    included.add(peer);
                }
            }
        }

        return [...this.dependencies].filter(([each]) => included.has(each));
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

    /** Read the runtimes the unit runs on: Bun alone once it or a dependency has a native addon. */
    runtimes(): Promise<readonly Runtime[]> {
        return this.#runtimes.get(async () => {
            // run on Bun alone with a native addon of its own
            const paths = await NpmUnit.paths(this.directory, this.#environment.files);
            if (paths.some((path) => PackageFile.isAddon(path))) {
                return [HOST_RUNTIME];
            }

            // run where every dependency runs
            const depended = await Promise.all(
                [...this.dependencies.values()].map(async (unit) => unit.runtimes()),
            );

            return Runtime.options.filter((runtime) =>
                depended.every((runtimes) => runtimes.includes(runtime)),
            );
        });
    }

    /** Build the unit in memory, converted to ES modules, with an output per runtime it runs on. */
    async build(store: (dependency: NpmUnit) => Promise<NpmDependency>): Promise<BuildReader> {
        // read the files, and the runtimes the unit runs on
        const files = await NpmUnit.files(this.directory, this.#environment.files);
        const runtimes = await this.runtimes();

        // write each runtime's conversion below the runtime
        const written = new Map<string, Uint8Array<ArrayBuffer>>();
        const writer = new BuildWriter(
            { write: async (path, bytes) => void written.set(path, bytes) },
            this.#environment.crypto,
        );
        const outputs: Record<string, OutputContent> = {};
        const required = new Map<string, NpmUnit>();
        for (const runtime of runtimes) {
            const { output, imported } = await this.#convert(files, runtime, writer);
            outputs[runtime] = output;
            for (const [each, unit] of imported) {
                required.set(each, unit);
            }
        }

        // store the units its modules import
        await Promise.all([...required.values()].map((unit) => store(unit)));
        const requires = Object.fromEntries(
            [...required]
                .map(([each, unit]) => [each, unit.release.package.version] as const)
                .toSorted(([left], [right]) => (left < right ? -1 : Number(left > right))),
        );
        const { integrity } = this.release;
        const manifest = await writer.finish({
            destack: DESTACK_VERSION,
            package: { ...this.release.package, integrity },
            role: "library",
            outputs,
            requires,
        });

        return new BuildReader(
            manifest,
            async (path) => new Blob([found(written, path)]).stream(),
            this.#environment.crypto,
        );
    }

    /** Write the unit for one runtime: published files for Bun, else converted ones. */
    async #convert(
        files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
        runtime: Runtime,
        writer: BuildWriter,
    ): Promise<{ readonly output: OutputContent; readonly imported: [string, NpmUnit][] }> {
        // keep the published files for the host, else convert them
        const converted =
            runtime === HOST_RUNTIME
                ? {
                      files,
                      exports: {},
                      imported: new Set([...this.dependencies.keys(), ...this.peers]),
                  }
                : await new NpmConversion(
                      this.release.package.name,
                      [...this.dependencies.keys()],
                      this.peers,
                      runtime,
                      this.#environment,
                  ).convert(files);
        await Promise.all(
            [...converted.files].map(([path, bytes]) => writer.write(`${runtime}/${path}`, bytes)),
        );

        // describe its exports, each importing the packages its converted modules import
        const imports = [...converted.imported].toSorted();
        const output: OutputContent = {
            runtime,
            emit: true,
            workloads: {},
            views: {},
            directory: runtime,
            exports: Object.fromEntries(
                Object.entries(converted.exports).map(([subpath, module]) => [
                    subpath,
                    { path: `${runtime}/${module}`, imports },
                ]),
            ),
            peers: Object.fromEntries(
                this.peers
                    .filter((peer) => converted.imported.has(peer))
                    .toSorted()
                    .map((peer) => [
                        peer,
                        { isOptional: PackageJson.isOptional(this.requirements, peer) },
                    ]),
            ),
        };

        return { output, imported: this.#provided(converted.imported) };
    }

    /** List an installed copy's file paths in order, leaving its node_modules out. */
    static async paths(directory: string, files: FileSystem): Promise<string[]> {
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
            throw BuildError.of(
                "DEPENDENCY_INVALID",
                `npm package folder link '${folder}' is invalid: npm packages must not link folders`,
            );
        }

        return paths;
    }

    /** Read every file of an installed copy by its path, leaving the copy's node_modules out. */
    static async files(
        directory: string,
        files: FileSystem,
    ): Promise<Map<string, Uint8Array<ArrayBuffer>>> {
        // read the files a bounded number at a time
        const paths = await NpmUnit.paths(directory, files);
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
