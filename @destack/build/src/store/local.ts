import { dirname, join } from "node:path";
import { LocalBlobStorage } from "@destack/bucket/bun";
import { BlobStore, eq } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { type Environment, type FileSystem, FileSystemError } from "@destack/environment";
import type { PackageFile } from "@destack/package/file";
import { BuildStore } from "./store.ts";
import { BuildError } from "../error/index.ts";
import { BuildReader, MANIFEST_PATH, PackageManifest } from "@destack/package/manifest";
import { BuildLayer, LAYER_INDEX, LAYER_TREES, LayerIndex } from "./layer.ts";
import { Toolchain } from "../cache/cache.ts";
import { DependencyTree, STAGING_DIRECTORY } from "../dependency/tree.ts";
import { Digest, present, schema, Uuid } from "@destack/schema";
import { HOST_RUNTIME, type Runtime } from "@destack/package/runtime";

/** The directory below a store's directory with its blobs, by digest. */
const FILES_DIRECTORY = "files";

/** The database file of a store of its own, which indexes its blobs. */
const DATABASE_FILE = "store.db";

/** The directory below a store's directory with the dependency trees of its builds, by manifest. */
const TREES_DIRECTORY = "trees";

/** The directory below a store's directory that compilers write builds in, on the blobs' volume. */
const STAGE_DIRECTORY = "stage";

/** The directory below a store's directory with the builds its linked layers share, by digest. */
const LAYERED_DIRECTORY = "layered";

/** The directory below a store's directory with its linked layers, by releases and runtimes. */
const LAYERS_DIRECTORY = "layers";

/** The suffix of a layered build's run tree while its links are written, moved in once whole. */
const STAGING_SUFFIX = ".staging";

/** The prefix of a root a directory names, forgotten once the directory is gone. */
const DIRECTORY_ROOT_PREFIX = "directory:";

/** The files placed at once: well below the default limit of 256 open files on macOS. */
const PLACE_BATCH = 64;

/** The file every process using a store locks shared. */
const STORE_LOCK = "store.lock";

/** The file a store's directory locks while one process opens and collects the store. */
const OPEN_LOCK = "open.lock";

/** The file with the time an idle open last collected the store, in milliseconds. */
const COLLECTED_FILE = "collected";

/** How often an idle open collects the store: daily, as Cargo's gc.auto.frequency defaults to. */
const COLLECT_MILLISECONDS = 24 * 60 * 60_000;

/** The most bytes a local store's blobs take: 10 GiB, three times a platform's 3 GB of builds. */
const MAX_BYTES = 10 * 1024 ** 3;

/** How long a store retains an unrooted build past its last use: a week, as Nx keeps its cache. */
const CACHE_MILLISECONDS = 7 * 24 * 60 * 60_000;

/** How often a store records a build's use: daily, a seventh of its retention. */
const USE_MILLISECONDS = 24 * 60 * 60_000;

/** The directory of Destack's caches below the person's cache directory. */
const CACHE_DIRECTORY = "destack";

/** What the person's store has builds for: development tools, or test runs. */
export type LocalStoreUse = "development" | "test";

/** How a layer keeps its builds and trees: in itself as shipped, or linked to its store's own. */
export type LayerLayout = "contained" | "linked";

/** The blobs, database, trees directory and environment of a store on this machine's disk. */
export interface LocalBuildStoreOptions {
    /** The blobs, kept in a directory, whose database declares the store's tables. */
    readonly blobs: BlobStore<LocalBlobStorage>;
    /** The directory with the dependency trees. */
    readonly directory: string;
    /** The environment the store runs in. */
    readonly environment: Environment<"files" | "process" | "host">;
    /** The read-only builds the store reads in place, such as a machine's shipped builds. */
    readonly layer?: BuildLayer;
}

/** A file a store writes to a path. */
export interface FilePlacement {
    /** The file. */
    readonly file: PackageFile;
    /** The path it is written to. */
    readonly destination: string;
}

/** A store of builds on this machine's disk, its blobs in a directory, which processes share. */
export class LocalBuildStore
    extends BuildStore<Environment<"files" | "process" | "host">, LocalBlobStorage>
    implements AsyncDisposable
{
    /** The directory with the store's blobs, database and trees. */
    readonly directory: string;
    /** The directory of the dependency trees its builds are materialized in, one per manifest. */
    readonly trees: string;
    /** The toolchain this process loaded, which the keys of its builds read. */
    readonly toolchain: Toolchain;
    /** What disposal releases: the database the store opened, and a temporary store's directory. */
    readonly #disposables: AsyncDisposableStack;
    /** The store that renews the builds' retention, absent when this one does. */
    readonly #maintainer: LocalBuildStore | undefined;
    /** The builds a view used, renewed by its maintainer once the view closes. */
    readonly #used = new Set<Digest>();

    /** Store builds in a directory's blobs indexed in a database, and what disposal releases. */
    private constructor(
        options: LocalBuildStoreOptions,
        disposables: AsyncDisposableStack,
        maintainer: LocalBuildStore | undefined,
    ) {
        // store the blobs, the database, the trees, the disposables and the store renewing retention
        super(options);
        this.directory = options.directory;
        this.trees = join(options.directory, TREES_DIRECTORY);
        this.toolchain = Toolchain.of(options.environment);
        this.#disposables = disposables;
        this.#maintainer = maintainer;
    }

    /** Store builds in blobs and an opened database, their trees below a directory. */
    static async of(options: LocalBuildStoreOptions): Promise<LocalBuildStore> {
        return await LocalBuildStore.#create(options, new AsyncDisposableStack());
    }

    /** Store a directory's builds, creating the directory of their trees. */
    static async #create(
        options: LocalBuildStoreOptions,
        disposables: AsyncDisposableStack,
        maintainer?: LocalBuildStore,
    ): Promise<LocalBuildStore> {
        const store = new LocalBuildStore(options, disposables, maintainer);
        await options.environment.files.mkdir(store.trees, { recursive: true });

        return store;
    }

    /** Open a store of its own in a directory: its blobs, and its database migrated. */
    static async #openDirectory(
        directory: string,
        environment: Environment<"files" | "process" | "host">,
        disposables: AsyncDisposableStack,
        maintainer?: LocalBuildStore,
    ): Promise<LocalBuildStore> {
        // open the blobs, then the database, closed on disposal
        const storage = await LocalBlobStorage.open(join(directory, FILES_DIRECTORY), environment, {
            synchronous: "off",
        });
        const database = await connectBunSqlite(
            join(directory, DATABASE_FILE),
            BuildStore.database,
            { environment },
        );
        disposables.defer(() => database.close());
        await database.migrate(BuildStore.database.tables);
        const blobs = new BlobStore({ storage, database, environment });

        return await LocalBuildStore.#create(
            { blobs, directory, environment },
            disposables,
            maintainer,
        );
    }

    /** View this store's directory through another environment, this store renewing retention. */
    async view(
        environment: Environment<"files" | "process" | "host">,
        scheduler?: Environment["scheduler"],
    ): Promise<LocalBuildStore> {
        // open the view, then release this store after it, as one operation in a world it runs outside
        const disposables = new AsyncDisposableStack();
        disposables.defer(() =>
            scheduler === undefined
                ? this.#release(view)
                : scheduler.run(() => this.#release(view)),
        );
        const view = await LocalBuildStore.#openDirectory(
            this.directory,
            environment,
            disposables,
            this,
        );

        return view;
    }

    /** Renew the builds a closed view used, then close this store. */
    async #release(view: LocalBuildStore): Promise<void> {
        for (const digest of view.#used) {
            await this.#use(digest);
        }
        await this[Symbol.asyncDispose]();
    }

    /** Store a build, retaining it a week past this use. */
    override async put(...stored: Parameters<BuildStore["put"]>): Promise<Digest> {
        const digest = await super.put(...stored);
        await this.#use(digest);

        return digest;
    }

    /** Open a stored build, retaining it a week past this use, as a cache does. */
    override async contents(digest: Digest): Promise<BuildReader> {
        await this.#use(digest);

        return super.contents(digest);
    }

    /** Lease a stored build a week on once a day it is used, retaining it and its files. */
    async #use(digest: Digest): Promise<void> {
        // record the use for the maintaining store to renew once this view closes
        if (this.#maintainer !== undefined) {
            this.#used.add(digest);

            return;
        }

        // read the stored build's lease, renewing one a day or more old, and none for a layered build read in place
        const now = this.environment.clock.now();
        const [leased] = await this.database
            .select({ until: BlobStore.table.leasedUntil })
            .from(BlobStore.table)
            .where(eq(BlobStore.table.digest, digest));
        if (leased !== undefined && leased.until - CACHE_MILLISECONDS <= now - USE_MILLISECONDS) {
            await this.lease(digest, new Date(now + CACHE_MILLISECONDS));
        }
    }

    /** Root some builds by a directory, until the directory is gone. */
    async rootDirectory(directory: string, manifests: ReadonlySet<string>): Promise<void> {
        await this.root(`${DIRECTORY_ROOT_PREFIX}${directory}`, manifests);
    }

    /** Create a directory to write a build in beside the blobs, which idle collection removes. */
    async stage(prefix: string): Promise<string> {
        // create a fresh directory below the store's stage
        const { files, random } = this.environment;
        const directory = join(this.directory, STAGE_DIRECTORY, `${prefix}${Uuid.v4(random)}`);
        await files.mkdir(directory, { recursive: true });

        return directory;
    }

    /** Store a build written in a directory the store consumes, moving its files into the blobs. */
    async move(directory: string, signal?: AbortSignal): Promise<Digest> {
        // store each missing file by moving it in, recorded under a lease, then remove what the directory has left
        const { files, crypto, clock } = this.environment;
        const source = await BuildReader.openDirectory(directory, files, crypto);
        const digest = await this.store(
            source,
            async (moved) => {
                const placements = [];
                for (const file of moved) {
                    placements.push(
                        await this.blobs.storage.move(join(directory, file.path), file.digest),
                    );
                }
                await this.blobs.record(placements, clock.now() + this.leaseMilliseconds);
            },
            signal,
        );
        await files.remove(directory, { recursive: true, force: true });

        return digest;
    }

    /** Write releases with what they run to a layer directory, as a distribution ships them. */
    async writeLayer(
        roots: readonly Digest[],
        runtimes: readonly Runtime[],
        directory: string,
        layout: LayerLayout = "contained",
    ): Promise<void> {
        // collect each root and the builds its emitted outputs' exports import on the shipped runtimes
        const builds = new Set<Digest>(roots);
        for (const root of roots) {
            const read = await this.get(root);
            const packageId = PackageManifest.package(read).id;
            const emitted = Object.values(read.outputs).filter(
                (output) => output.emit && runtimes.includes(output.runtime),
            );
            for (const output of emitted) {
                const exported = await this.closure(
                    root,
                    packageId,
                    output.runtime,
                    undefined,
                    "export",
                );
                for (const reached of exported) {
                    builds.add(reached);
                }
            }
        }

        // write each build missing its manifest, which a build writes last, in the store's layered builds for a linked layer, else in the layer
        const { files } = this.environment;
        const pool = layout === "linked" ? join(this.directory, LAYERED_DIRECTORY) : undefined;
        await files.mkdir(directory, { recursive: true });
        const readers = new Map<Digest, BuildReader>();
        for (const digest of builds) {
            const reader = await this.contents(digest);
            readers.set(digest, reader);
            if (pool === undefined) {
                await LocalBuildStore.#writeBuild(reader, join(directory, digest), files);
            } else {
                await LocalBuildStore.#writePooled(reader, join(pool, digest), files);
                if (!(await files.exists(join(directory, digest)))) {
                    await files.symlink(join(pool, digest), join(directory, digest));
                }
            }
        }

        // remove the builds left out
        const kept = new Set<string>(builds);
        const stale = (await files.readDirectory(directory)).filter(
            (entry) =>
                (entry.kind === "directory" || entry.kind === "symlink") &&
                entry.name !== LAYER_TREES &&
                !kept.has(entry.name),
        );
        for (const entry of stale) {
            await files.remove(join(directory, entry.name), { recursive: true });
        }

        // lay out the releases' trees once, in the store's own trees for a linked layer, an empty directory for no releases
        const trees = pool === undefined ? join(directory, LAYER_TREES) : this.trees;
        await files.mkdir(trees, { recursive: true });
        if (pool !== undefined && !(await files.exists(join(directory, LAYER_TREES)))) {
            await files.symlink(trees, join(directory, LAYER_TREES));
        }
        const released = await Promise.all(
            roots.map(async (digest) => ({
                kind: "destack" as const,
                package: PackageManifest.package(await this.get(digest)),
                manifest: digest,
            })),
        );
        await DependencyTree.materialize(this, released, trees);

        // link each root's host run tree as its node_modules, so the host runs it in place
        const isHosting = runtimes.includes(HOST_RUNTIME);
        for (const root of isHosting ? roots : []) {
            const reader = present(readers.get(root), `the layered build ${root}`);
            const isRunning = Object.values(reader.manifest.outputs).some(
                (output) => output.runtime === HOST_RUNTIME,
            );
            const modules = join(directory, root, "node_modules");
            if (isRunning && !(await files.exists(modules))) {
                const tree = await this.runTree(reader, root, HOST_RUNTIME, trees);
                const staging = `${modules}${STAGING_SUFFIX}`;
                await files.remove(staging, { recursive: true, force: true });
                await tree.link(staging, files);
                await files.rename(staging, modules);
            }
        }

        // remove the trees of builds a contained layer no longer has, leaving the store's own trees
        for (const entry of pool === undefined ? await files.readDirectory(trees) : []) {
            const built = DependencyTree.manifestOf(entry.name);
            if (built === undefined || !builds.has(built)) {
                await files.remove(join(trees, entry.name), { recursive: true, force: true });
            }
        }

        // write the index of its builds and their files last
        await BuildLayer.writeIndex(directory, roots, readers, files);
    }

    /** Write a linked layer of releases once, keyed with the runtimes, returning its directory. */
    async writeLinkedLayer(
        roots: readonly Digest[],
        runtimes: readonly Runtime[],
    ): Promise<string> {
        // leave a layer written before
        const { files, crypto, process } = this.environment;
        const key = await Digest.json({ releases: roots, runtimes }, crypto);
        const directory = join(this.directory, LAYERS_DIRECTORY, key);
        if (await files.exists(directory)) {
            return directory;
        }

        // write it under this process's name, then move it in, leaving one another process moved in first
        const written = `${directory}.${process.id}`;
        await files.remove(written, { recursive: true, force: true });
        await files.mkdir(dirname(written), { recursive: true });
        await this.writeLayer(roots, runtimes, written, "linked");
        await files.rename(written, directory).catch(async (error: unknown) => {
            if (!FileSystemError.is(error, "ALREADY_EXISTS")) {
                throw error;
            }
            await files.remove(written, { recursive: true });
        });

        return directory;
    }

    /** Write a build's directory unless its manifest, which a build writes last, is there. */
    static async #writeBuild(reader: BuildReader, build: string, files: FileSystem): Promise<void> {
        if (!(await files.exists(join(build, MANIFEST_PATH)))) {
            await files.remove(build, { recursive: true, force: true });
            await reader.writeDirectory(build, files, await BuildLayer.shipped(reader));
        }
    }

    /** Write a build into the store's layered builds once, unless another writer did first. */
    static async #writePooled(
        reader: BuildReader,
        build: string,
        files: FileSystem,
    ): Promise<void> {
        // leave a build the pool has
        if (await files.exists(join(build, MANIFEST_PATH))) {
            return;
        }

        // stage it beside its place, then move it in
        await files.mkdir(dirname(build), { recursive: true });
        const staging = await files.makeTemporaryDirectory(".staging.", {
            directory: dirname(build),
        });
        try {
            await reader.writeDirectory(
                join(staging, "build"),
                files,
                await BuildLayer.shipped(reader),
            );
            await files.rename(join(staging, "build"), build).catch(async (error: unknown) => {
                if (!(await files.exists(join(build, MANIFEST_PATH)))) {
                    throw error;
                }
            });
        } finally {
            await files.remove(staging, { recursive: true, force: true });
        }
    }

    /** Lay out the tree a build's output runs with on a runtime, a layered one in place. */
    async runTree(
        built: BuildReader,
        digest: Digest,
        runtime: Runtime,
        directory: string = this.trees,
    ): Promise<DependencyTree> {
        // read the closure of every export, refusing a build the store lacks
        const { manifest } = built;
        const closure = await this.closure(
            digest,
            PackageManifest.package(manifest).id,
            runtime,
            async (needed) => {
                const found = await this.find(needed);
                if (found === undefined) {
                    throw BuildError.of(
                        "DEPENDENCY_UNRESOLVED",
                        `build ${needed} of ${manifest.package.name} is missing from the store`,
                    );
                }

                return found;
            },
            "export",
        );

        // place each closure build's output once
        const output = present(
            Object.values(manifest.outputs).find((each) => each.runtime === runtime),
            `the ${runtime} output of ${manifest.package.name}`,
        );
        const dependencies = Object.values(output.dependencies).filter((dependency) =>
            closure.has(dependency.manifest),
        );

        return await DependencyTree.materialize(this, dependencies, directory, {
            run: { runtime, closure },
        });
    }

    /** Write files to paths, cloning stored blobs and the layer's files. */
    async place(placements: readonly FilePlacement[]): Promise<void> {
        // create each directory once, and find the files the store's own blobs lack
        const { files } = this.environment;
        const directories = new Set(placements.map(({ destination }) => dirname(destination)));
        for (const directory of [...directories].toSorted()) {
            await files.mkdir(directory, { recursive: true });
        }
        const lacking = new Set(
            await this.blobs.missing(placements.map(({ file }) => file.digest)),
        );

        // clone a stored file's blocks, and place any other through the layered build holding it, a batch at a time
        for (let start = 0; start < placements.length; start += PLACE_BATCH) {
            await Promise.all(
                placements
                    .slice(start, start + PLACE_BATCH)
                    .map(({ file, destination }) =>
                        lacking.has(file.digest)
                            ? this.placeLayered(file, destination, files)
                            : this.blobs.storage.copy(file.digest, destination),
                    ),
            );
        }
    }

    /** Forget deleted directories' roots, collect, then delete deleted builds' trees. */
    async #collectIdle(): Promise<void> {
        // forget each root of a directory that no longer exists
        for (const name of (await this.roots()).keys()) {
            const directory = name.startsWith(DIRECTORY_ROOT_PREFIX)
                ? name.slice(DIRECTORY_ROOT_PREFIX.length)
                : undefined;
            if (directory !== undefined && !(await this.environment.files.exists(directory))) {
                await this.forget(name);
            }
        }

        // delete what crashed materializations and compilers left staged, and what interrupted blob writes left
        const { files } = this.environment;
        await files.remove(join(this.trees, STAGING_DIRECTORY), { recursive: true, force: true });
        await files.remove(join(this.directory, STAGE_DIRECTORY), { recursive: true, force: true });
        await this.blobs.storage.clean();

        // drop the unused cache entries, fit the size, then delete what nothing retains at once, as no process reads
        await this.expire();
        await this.evict(MAX_BYTES);
        await this.blobs.collect(0);

        // delete the trees of the builds whose manifests the store no longer has, whole or in part
        const entries = await files.readDirectory(this.trees);
        const trees = entries
            .filter((entry) => entry.kind === "directory" && entry.name !== STAGING_DIRECTORY)
            .map((entry) => entry.name);
        for (const tree of trees) {
            const manifest = DependencyTree.manifestOf(tree);
            if (manifest === undefined || !(await this.has(manifest))) {
                await files.remove(join(this.trees, tree), { recursive: true, force: true });
            }
        }

        // delete the layered builds the store no longer has, and what interrupted writes left
        const layered = join(this.directory, LAYERED_DIRECTORY);
        for (const entry of await LocalBuildStore.#entries(layered, files)) {
            const digest = Digest.safeParse(entry);
            if (!digest.success || !(await this.has(digest.data))) {
                await files.remove(join(layered, entry), { recursive: true, force: true });
            }
        }

        // delete the layers linking a build the store no longer has, and what interrupted writes left
        const layers = join(this.directory, LAYERS_DIRECTORY);
        for (const entry of await LocalBuildStore.#entries(layers, files)) {
            const index = await files
                .readTextFile(join(layers, entry, LAYER_INDEX))
                .then((text) => LayerIndex.parse(JSON.parse(text)))
                .catch(FileSystemError.absent);
            const builds = Object.keys(index?.builds ?? {}).map((each) => Digest.parse(each));
            const isComplete = index !== undefined && (await this.#hasAll(builds));
            if (!isComplete) {
                await files.remove(join(layers, entry), { recursive: true, force: true });
            }
        }
    }

    /** List a directory's entries by name, none for a directory that does not exist. */
    static async #entries(directory: string, files: FileSystem): Promise<string[]> {
        const entries = await files.readDirectory(directory).catch(FileSystemError.absent);

        return (entries ?? []).map((entry) => entry.name);
    }

    /** Report whether the store has every build of a list. */
    async #hasAll(builds: readonly Digest[]): Promise<boolean> {
        for (const build of builds) {
            if (!(await this.has(build))) {
                return false;
            }
        }

        return true;
    }

    /** Collect an idle store a day after its last collection, recording when it did. */
    async #collectIdleIfDue(path: string): Promise<void> {
        // read when the store last collected, never for a new store
        const { files, clock } = this.environment;
        const now = clock.now();
        const collected = (await files.exists(path))
            ? schema.int().parse(Number(await files.readTextFile(path)))
            : undefined;

        // collect once the day since passed, and record it
        if (collected === undefined || now - collected >= COLLECT_MILLISECONDS) {
            await this.#collectIdle();
            await files.writeTextFile(path, String(now));
        }
    }

    /** Open the store in a directory, collecting it daily when no other process has it open. */
    static async open(
        directory: string,
        environment: Environment<"files" | "process" | "host">,
    ): Promise<LocalBuildStore> {
        // lock the directory while opening and collecting, so no process builds into a store being collected
        const { files } = environment;
        await files.mkdir(directory, { recursive: true });
        const held = await files.lock(join(directory, OPEN_LOCK), "exclusive");
        try {
            // learn whether another process has the store open, as openers wait here
            const location = await files.realPath(directory);
            const idle = await files.tryLock(join(location, STORE_LOCK), "exclusive");
            await idle?.close();

            // open the store sharing its lock, then collect it while no other process builds into it
            const disposables = new AsyncDisposableStack();
            try {
                const shared = await files.lock(join(location, STORE_LOCK), "shared");
                disposables.defer(() => shared.close());
                const store = await LocalBuildStore.#openDirectory(
                    location,
                    environment,
                    disposables,
                );
                if (idle !== undefined) {
                    await store.#collectIdleIfDue(join(location, COLLECTED_FILE));
                }

                return store;
            } catch (error) {
                // release the lock and whatever opened once opening fails
                await disposables.disposeAsync();
                throw error;
            }
        } finally {
            await held.close();
        }
    }

    /** Open the person's store for one use, in their cache directory or the environment's. */
    static async user(
        use: LocalStoreUse,
        environment: Environment<"files" | "process" | "host">,
    ): Promise<LocalBuildStore> {
        // take the cache directory the environment names, as NX_CACHE_DIRECTORY does, else the person's
        const { DESTACK_CACHE_DIRECTORY: named } = environment.process.variables;
        if (named === "") {
            throw BuildError.of("CACHE_DIRECTORY_INVALID", "DESTACK_CACHE_DIRECTORY is empty");
        }
        const cache = named ?? join(await environment.files.cacheDirectory(), CACHE_DIRECTORY);

        return await LocalBuildStore.open(join(cache, use), environment);
    }

    /** Open a store of one run's builds, removed on disposal. */
    static async temporary(
        environment: Environment<"files" | "process" | "host">,
    ): Promise<LocalBuildStore> {
        // open a store in a new directory, both released on disposal
        const { files } = environment;
        const temporary = await files.makeTemporaryDirectory("destack-store-");
        const directory = await files.realPath(temporary);
        const disposables = new AsyncDisposableStack();
        disposables.defer(() => files.remove(directory, { recursive: true }));

        return await LocalBuildStore.#openDirectory(directory, environment, disposables);
    }

    /** Close the database the store opened, removing a temporary store. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#disposables.disposeAsync();
    }
}
