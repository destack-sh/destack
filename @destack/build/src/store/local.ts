import { dirname, join } from "node:path";
import { LocalBlobStorage } from "@destack/bucket/bun";
import { BlobStore, eq } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { type Environment, type FileSystem, FileSystemError } from "@destack/environment";
import { type PackageFile, PackagePath } from "@destack/package/file";
import { BuildObject, BuildStore, CACHE_MILLISECONDS, USE_MILLISECONDS } from "./store.ts";
import { BuildError } from "../error/index.ts";
import { BuildReader, Lock, MANIFEST_PATH, PackageManifest } from "@destack/package/manifest";
import {
    BuildLayer,
    LAYER_BLOBS,
    LAYER_FORMAT,
    LAYER_INDEX,
    LAYER_TREES,
    LayerIndex,
} from "@destack/package/manifest";
import { DependencyTree, STAGING_DIRECTORY } from "../dependency/tree.ts";
import { Digest, found, JsonValue, present, schema, Uuid } from "@destack/schema";
import { HOST_RUNTIME, type Runtime } from "@destack/package/runtime";

/** The directory below a store's directory with its blobs, by digest. */
const FILES_DIRECTORY = "files";

/** The database file of a standalone store, which indexes its blobs. */
const DATABASE_FILE = "store.db";

/** The directory below a store's directory with the dependency trees of its builds, by manifest. */
const TREES_DIRECTORY = "trees";

/** The directory below a store's directory that compilers write builds in, on the blobs' volume. */
const STAGE_DIRECTORY = "stage";

/** The directory below a store's directory with the builds its linked layers share, by digest. */
const LAYERED_DIRECTORY = "layered";

/** The directory below a store's directory with its linked layers, by releases and runtimes. */
const LAYERS_DIRECTORY = "layers";

/** The suffix of a lock's run directory while it is written, moved in once whole. */
const STAGING_SUFFIX = ".staging";

/** The prefix of a directory's root, forgotten once the directory is gone. */
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

/** The directory of Destack's caches below the person's cache directory. */
const CACHE_DIRECTORY = "destack";

/** What the person's store has builds for: development tools, or test runs. */
export type LocalStoreUse = "development" | "test";

/** How a layer lays out its builds: linked to its store's pooled builds, or as blobs by digest. */
export type LayerLayout = "linked" | "addressed";

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

    /** Open a standalone store in a directory: its blobs, and its database migrated. */
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
        // release this store after the view, as one operation of the scheduler's world
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
        }
        // renew a stored build's lease a day or more old, a layered build having none
        else {
            const now = this.environment.clock.now();
            const [leased] = await this.database
                .select({ until: BlobStore.table.leasedUntil })
                .from(BlobStore.table)
                .where(eq(BlobStore.table.digest, digest));
            const isDue =
                leased !== undefined && leased.until - CACHE_MILLISECONDS <= now - USE_MILLISECONDS;
            if (isDue) {
                await this.lease(digest, new Date(now + CACHE_MILLISECONDS));
            }
        }
    }

    /** Root some builds by a directory, until the directory is gone. */
    async rootDirectory(directory: string, manifests: ReadonlySet<Digest>): Promise<void> {
        await this.retain(`${DIRECTORY_ROOT_PREFIX}${directory}`, manifests);
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
        // store each missing file by moving it in, recorded under a lease
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

        // remove what the directory has left
        await files.remove(directory, { recursive: true, force: true });

        return digest;
    }

    /** The store's layered builds of this layer format, by digest, which linked layers share. */
    get #pool(): string {
        return join(this.directory, LAYERED_DIRECTORY, String(LAYER_FORMAT));
    }

    /** Write locked releases to a layer: whole builds for a computer, run files for the cloud. */
    async writeLayer(
        locks: readonly Digest[],
        runtimes: readonly Runtime[],
        directory: string,
        layout: LayerLayout,
    ): Promise<void> {
        // read each lock and its builds
        const shipped = new Map<Digest, Lock>();
        const builds = new Set<Digest>();
        for (const digest of locks) {
            const lock = await this.lock(digest);
            shipped.set(digest, lock);
            for (const build of Lock.builds(lock)) {
                builds.add(build);
            }
        }

        // write an addressed layer's blobs alone, what the runtimes run, as the cloud only runs
        if (layout === "addressed") {
            await this.#writeAddressed(shipped, runtimes, directory);

            return;
        }

        // write each build whole once in the store's layered builds, and link it from the layer
        const { files } = this.environment;
        const pool = this.#pool;
        await files.mkdir(directory, { recursive: true });
        const readers = new Map<Digest, BuildReader>();
        for (const digest of builds) {
            const reader = await this.contents(digest);
            readers.set(digest, reader);
            await this.#writePooled(reader, join(pool, digest));
            if (!(await files.exists(join(directory, digest)))) {
                await files.symlink(join(pool, digest), join(directory, digest));
            }
        }

        // remove the builds and run directories left out
        const kept = new Set<string>([...builds, ...shipped.keys()]);
        const stale = (await files.readDirectory(directory)).filter(
            (entry) =>
                (entry.kind === "directory" || entry.kind === "symlink") &&
                entry.name !== LAYER_TREES &&
                !kept.has(entry.name),
        );
        for (const entry of stale) {
            await files.remove(join(directory, entry.name), { recursive: true });
        }

        // lay out the releases' trees once for the host in the store's own trees, linked from the layer
        const isHosting = runtimes.includes(HOST_RUNTIME);
        const trees = this.trees;
        await files.mkdir(trees, { recursive: true });
        if (!(await files.exists(join(directory, LAYER_TREES)))) {
            await files.symlink(trees, join(directory, LAYER_TREES));
        }
        for (const lock of isHosting ? shipped.values() : []) {
            const releases = await Lock.releases(lock, (digest) => this.get(digest));
            const resolve = (build: Digest, name: string) => Lock.resolve(lock, build, name);
            await DependencyTree.materialize(this, releases, resolve, trees, { kind: "compile" });
        }

        // write each hosted lock's root with its run tree in a directory of the lock's own
        for (const [digest, lock] of isHosting ? shipped : []) {
            const reader = found(readers, lock.root);
            const isRunning = Object.values(reader.manifest.outputs).some(
                (output) => output.runtime === HOST_RUNTIME,
            );
            const run = join(directory, digest);
            if (isRunning && !(await files.exists(run))) {
                await this.#writeRun(reader, await this.runTree(lock, HOST_RUNTIME, trees), run);
            }
        }

        // index each build's manifest and files last
        const sizes: LayerIndex["builds"] = {};
        const placed: LayerIndex["files"] = {};
        for (const [digest, reader] of readers) {
            sizes[digest] = PackageManifest.serialize(reader.manifest).byteLength;
            for (const file of await reader.distributed()) {
                placed[file.digest] ??= { path: `${digest}/${file.path}`, size: file.size };
            }
        }
        await BuildLayer.writeIndex(directory, shipped, sizes, placed, files);
    }

    /** Write a linked layer of locked releases once, keyed with the runtimes. */
    async writeLinkedLayer(
        locks: readonly Digest[],
        runtimes: readonly Runtime[],
    ): Promise<string> {
        // leave a layer written before in this format
        const { files, crypto, process } = this.environment;
        const key = await Digest.json({ format: LAYER_FORMAT, releases: locks, runtimes }, crypto);
        const directory = join(this.directory, LAYERS_DIRECTORY, key);
        if (await files.exists(directory)) {
            return directory;
        }

        // write it under this process's name, then move it in, leaving one another process moved in first
        const written = `${directory}.${process.id}`;
        await files.remove(written, { recursive: true, force: true });
        await files.mkdir(dirname(written), { recursive: true });
        await this.writeLayer(locks, runtimes, written, "linked");
        await files.rename(written, directory).catch(async (error: unknown) => {
            if (!FileSystemError.is(error, "FILE_EXISTS")) {
                throw error;
            }
            await files.remove(written, { recursive: true });
        });

        return directory;
    }

    /** Write what the locks' runtimes run as blobs by digest, the index last. */
    async #writeAddressed(
        locks: ReadonlyMap<Digest, Lock>,
        runtimes: readonly Runtime[],
        directory: string,
    ): Promise<void> {
        // start from an empty directory of blobs
        const { files } = this.environment;
        const blobs = join(directory, LAYER_BLOBS);
        await files.remove(directory, { recursive: true, force: true });
        await files.mkdir(blobs, { recursive: true });

        // list the objects a pull of the locks for the runtimes reads
        const roots = [...locks.keys()].map((digest) => ({ kind: "lock", digest }) as const);
        const objects = await BuildObject.of(this, roots, runtimes);
        const wanted = new Set(
            objects.filter((object) => object.kind === "file").map((object) => object.digest),
        );

        // write each build's manifest at its digest, and list the wanted files of it
        const sizes: LayerIndex["builds"] = {};
        const placed: LayerIndex["files"] = {};
        const placements: FilePlacement[] = [];
        for (const { digest } of objects.filter((object) => object.kind === "manifest")) {
            const reader = await this.contents(digest);
            const manifest = PackageManifest.serialize(reader.manifest);
            await files.writeFile(join(blobs, digest), manifest);
            sizes[digest] = manifest.byteLength;
            for (const file of await reader.distributed()) {
                if (wanted.has(file.digest) && placed[file.digest] === undefined) {
                    placements.push({ file, destination: join(blobs, file.digest) });
                    placed[file.digest] = {
                        path: `${LAYER_BLOBS}/${file.digest}`,
                        size: file.size,
                    };
                }
            }
        }

        // clone the listed files, then write the index
        await this.place(placements);
        await BuildLayer.writeIndex(directory, locks, sizes, placed, files);
    }

    /** Write a lock's root and its run tree in a directory, unless another process did first. */
    async #writeRun(reader: BuildReader, tree: DependencyTree, run: string): Promise<void> {
        // stage it under this process's name
        const { files, process } = this.environment;
        const staging = `${run}${STAGING_SUFFIX}.${process.id}`;
        await files.remove(staging, { recursive: true, force: true });
        await this.#writeDirectory(reader, staging, await reader.distributed());
        await tree.link(join(staging, "node_modules"), files);

        // move it in, leaving one another process moved in first
        await files.rename(staging, run).catch(async (error: unknown) => {
            if (!FileSystemError.is(error, "FILE_EXISTS")) {
                throw error;
            }
            await files.remove(staging, { recursive: true });
        });
    }

    /** Write a build into the store's layered builds once, unless another writer did first. */
    async #writePooled(reader: BuildReader, build: string): Promise<void> {
        // leave a build the pool has
        const { files } = this.environment;
        if (await files.exists(join(build, MANIFEST_PATH))) {
            return;
        }

        // stage it beside its place, then move it in
        await files.mkdir(dirname(build), { recursive: true });
        const staging = await files.makeTemporaryDirectory(".staging.", {
            directory: dirname(build),
        });
        try {
            // write every file, as linked layers of any runtimes share the pooled build
            await this.#writeDirectory(reader, join(staging, "build"), await reader.distributed());
            await files.rename(join(staging, "build"), build).catch(async (error: unknown) => {
                if (!(await files.exists(join(build, MANIFEST_PATH)))) {
                    throw error;
                }
            });
        } finally {
            await files.remove(staging, { recursive: true, force: true });
        }
    }

    /** Clone some files of a build into a new directory, its manifest last. */
    async #writeDirectory(
        reader: BuildReader,
        directory: string,
        selected: readonly PackageFile[],
    ): Promise<void> {
        // require a new directory
        const { files } = this.environment;
        await files.mkdir(directory);
        await this.place(
            selected.map((file) => ({
                file,
                destination: join(directory, PackagePath.parse(file.path)),
            })),
        );

        // make the build readable only after every file is written
        const manifest = PackageManifest.serialize(reader.manifest);
        await files.writeFile(join(directory, MANIFEST_PATH), manifest, { createNew: true });
    }

    /** Lay out the tree a lock's root runs with on a runtime, a layered one in place. */
    async runTree(
        lock: Lock,
        runtime: Runtime,
        directory: string = this.trees,
    ): Promise<DependencyTree> {
        // read the closure of every export, refusing a build the store lacks
        const closure = await this.closure(
            lock,
            runtime,
            async (needed) => {
                const stored = await this.find(needed);
                if (stored === undefined) {
                    throw BuildError.of(
                        "DEPENDENCY_INVALID",
                        `build '${needed}' of lock root '${lock.root}' is invalid: it is missing from the store`,
                    );
                }

                return stored;
            },
            "export",
        );

        // place each closure build's output once
        const releases = await Lock.releases(lock, (digest) => this.get(digest));
        const resolve = (build: Digest, name: string) => Lock.resolve(lock, build, name);

        return await DependencyTree.materialize(this, releases, resolve, directory, {
            kind: "run",
            runtime,
            closure,
        });
    }

    /** Write files to paths, cloning stored blobs and the layer's files. */
    async place(placements: readonly FilePlacement[]): Promise<void> {
        // create each directory once
        const { files } = this.environment;
        const directories = new Set(placements.map(({ destination }) => dirname(destination)));
        for (const directory of [...directories].toSorted()) {
            await files.mkdir(directory, { recursive: true });
        }

        // find where each file lies, then clone it a batch at a time
        const located = await this.locate(placements.map(({ file }) => file.digest));
        for (let start = 0; start < placements.length; start += PLACE_BATCH) {
            await Promise.all(
                placements.slice(start, start + PLACE_BATCH).map(({ file, destination }) => {
                    const location = present(
                        located.get(file.digest),
                        `package file '${file.path}'`,
                    );

                    return location.kind === "layered"
                        ? this.placeLayered(file, destination, files)
                        : this.blobs.storage.copy(location, destination);
                }),
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

        // delete what crashed materializations, compilers and blob writes left
        const { files } = this.environment;
        await files.remove(join(this.trees, STAGING_DIRECTORY), { recursive: true, force: true });
        await files.remove(join(this.directory, STAGE_DIRECTORY), { recursive: true, force: true });
        await this.blobs.storage.clean();

        // drop the unused cache entries and fit the size, then delete what nothing retains
        await this.expire();
        await this.evict(MAX_BYTES);
        await this.blobs.collect(0);

        // delete the trees of the builds whose manifests the store no longer has, whole or in part
        const entries = await files.readDirectory(this.trees);
        const trees = entries
            .filter((entry) => entry.kind === "directory" && entry.name !== STAGING_DIRECTORY)
            .map((entry) => ({
                name: entry.name,
                manifest: DependencyTree.manifestOf(entry.name),
            }));
        const manifests = await this.#stored(trees.map((tree) => tree.manifest));
        for (const tree of trees) {
            if (tree.manifest === undefined || !manifests.has(tree.manifest)) {
                await files.remove(join(this.trees, tree.name), { recursive: true, force: true });
            }
        }

        // delete the layered builds of other layer formats
        const layered = join(this.directory, LAYERED_DIRECTORY);
        for (const entry of await LocalBuildStore.#entries(layered, files)) {
            if (entry !== String(LAYER_FORMAT)) {
                await files.remove(join(layered, entry), { recursive: true, force: true });
            }
        }

        // delete the layered builds the store no longer has, and what interrupted writes left
        const pooled = (await LocalBuildStore.#entries(this.#pool, files)).map((name) => ({
            name,
            digest: Digest.safeParse(name).data,
        }));
        const kept = await this.#stored(pooled.map((entry) => entry.digest));
        for (const entry of pooled) {
            if (entry.digest === undefined || !kept.has(entry.digest)) {
                await files.remove(join(this.#pool, entry.name), { recursive: true, force: true });
            }
        }

        // delete the layers linking a build the store no longer has
        const layers = join(this.directory, LAYERS_DIRECTORY);
        for (const entry of await LocalBuildStore.#entries(layers, files)) {
            const text = await files
                .readTextFile(join(layers, entry, LAYER_INDEX))
                .catch(FileSystemError.absent);
            const written =
                text === undefined ? undefined : JsonValue.object(JsonValue.parse(text));
            const index =
                written?.["formatVersion"] === LAYER_FORMAT ? LayerIndex.parse(written) : undefined;
            const builds = Object.keys(index?.builds ?? {}).map((each) => Digest.parse(each));
            const isComplete =
                index !== undefined && (await this.has(builds)).every((isStored) => isStored);
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

    /** Find which of some blobs the store has, at once, leaving out absent digests. */
    async #stored(digests: readonly (Digest | undefined)[]): Promise<ReadonlySet<Digest>> {
        const given = digests.filter((digest) => digest !== undefined);
        const isStored = await this.has(given);

        return new Set(given.filter((_digest, index) => isStored[index] === true));
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
        // lock the directory while one process opens and collects the store
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
            throw BuildError.of(
                "CACHE_DIRECTORY_INVALID",
                "DESTACK_CACHE_DIRECTORY is invalid: it is empty",
            );
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
