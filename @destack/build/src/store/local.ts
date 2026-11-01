import { dirname, join } from "node:path";
import { LocalBlobStorage } from "@destack/bucket/bun";
import { BlobStore, blobLease, eq } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { locateWorkspace } from "@destack/check/workspace";
import type { Environment } from "@destack/environment";
import type { PackageFile } from "@destack/package/file";
import { BuildStore } from "./store.ts";
import { BuildError } from "../error/index.ts";
import { BuildReader, MANIFEST_PATH, PackageManifest } from "@destack/package/manifest";
import { BuildLayer, LAYER_TREES } from "./layer.ts";
import { Toolchain } from "../cache/cache.ts";
import { DependencyTree, STAGING_DIRECTORY } from "../dependency/tree.ts";
import { Digest, schema, Uuid } from "@destack/schema";
import type { Runtime } from "@destack/package/runtime";

/** The directory below a store's directory with its blobs, by digest. */
const FILES_DIRECTORY = "files";

/** The database file of a store of its own, which indexes its blobs. */
const DATABASE_FILE = "store.db";

/** The directory below a store's directory with the dependency trees of its builds, by manifest. */
const TREES_DIRECTORY = "trees";

/** The directory below a store's directory that compilers write builds in, on the blobs' volume. */
const STAGE_DIRECTORY = "stage";

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

/** How long a store retains an unrooted build past its last use: a week, as Nx keeps its cache. */
const CACHE_MILLISECONDS = 7 * 24 * 60 * 60_000;

/** How often a store records a build's use: daily, a seventh of its retention. */
const USE_MILLISECONDS = 24 * 60 * 60_000;

/** The cache directory below a checkout's root, where find-cache-dir places tools' caches. */
const CHECKOUT_CACHE = join("node_modules", ".cache", "destack");

/** What a checkout's store has builds for: development tools, or test runs. */
export type LocalStoreUse = "development" | "test";

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
        // open the view, then release this store after it, as one piece of host work in a world it runs outside
        const disposables = new AsyncDisposableStack();
        disposables.defer(() =>
            scheduler === undefined
                ? this.#release(view)
                : scheduler.track(() => this.#release(view), "host"),
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

    /** Lease a build a week on once a day it is used, retaining it and its files. */
    async #use(digest: Digest): Promise<void> {
        // record the use for the maintaining store to renew once this view closes
        if (this.#maintainer !== undefined) {
            this.#used.add(digest);

            return;
        }

        // read the build's lease, renewing one a day or more old
        const now = this.environment.clock.now();
        const [leased] = await this.database
            .select({ until: blobLease.until })
            .from(blobLease)
            .where(eq(blobLease.digest, digest));
        if (leased === undefined || leased.until - CACHE_MILLISECONDS <= now - USE_MILLISECONDS) {
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
        // keep each missing file by moving it, then remove what the directory has left
        const { files, crypto } = this.environment;
        const source = await BuildReader.openDirectory(directory, files, crypto);
        const digest = await this.keep(
            source,
            async (file) => {
                await this.blobs.storage.move(join(directory, file.path), file.digest);
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

        // write each build missing its manifest, which a build writes last, and remove the builds left out
        const { files } = this.environment;
        await files.mkdir(directory, { recursive: true });
        const readers = new Map<Digest, BuildReader>();
        for (const digest of builds) {
            const build = join(directory, digest);
            const reader = await this.contents(digest);
            readers.set(digest, reader);
            if (!(await files.exists(join(build, MANIFEST_PATH)))) {
                await files.remove(build, { recursive: true, force: true });
                await reader.writeDirectory(build, files);
            }
        }
        const kept = new Set<string>(builds);
        const stale = (await files.readDirectory(directory)).filter(
            (entry) =>
                entry.kind === "directory" && entry.name !== LAYER_TREES && !kept.has(entry.name),
        );
        for (const entry of stale) {
            await files.remove(join(directory, entry.name), { recursive: true });
        }

        // lay out the releases' trees once, an empty directory for no releases, removing those of builds it no longer has
        const trees = join(directory, LAYER_TREES);
        await files.mkdir(trees, { recursive: true });
        const released = await Promise.all(
            roots.map(async (digest) => ({
                kind: "destack" as const,
                package: PackageManifest.package(await this.get(digest)),
                manifest: digest,
            })),
        );
        await DependencyTree.materialize(this, released, trees);
        for (const entry of await files.readDirectory(trees)) {
            const built = DependencyTree.manifestOf(entry.name);
            if (built === undefined || !builds.has(built)) {
                await files.remove(join(trees, entry.name), { recursive: true, force: true });
            }
        }

        // write the index of its builds and their files last
        await BuildLayer.writeIndex(directory, roots, readers, files);
    }

    /** Write files to paths, cloning stored blobs and layered files where the layer keeps them. */
    async place(placements: readonly FilePlacement[]): Promise<void> {
        // create each directory once, and find the files the store's own blobs lack
        const { files } = this.environment;
        const directories = new Set(placements.map(({ destination }) => dirname(destination)));
        for (const directory of [...directories].toSorted()) {
            await files.mkdir(directory, { recursive: true });
        }
        const lacking = new Set(
            await this.blobs.storage.missing(placements.map(({ file }) => file.digest)),
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

        // drop the unused cache entries, then delete the builds and blobs nothing retains
        await this.expire();
        await this.blobs.collect();

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

    /** Open the cache store of the checkout enclosing a directory, with builds for one use. */
    static async checkout(
        directory: string,
        use: LocalStoreUse,
        environment: Environment<"files" | "process" | "host">,
    ): Promise<LocalBuildStore> {
        // find the workspace root past a link to its lockfile
        const { files } = environment;
        const located = await locateWorkspace(directory, files);
        const root = dirname(await files.realPath(join(located, "bun.lock")));

        // open the use's store in the cache directory the environment sets, as NX_CACHE_DIRECTORY does, else the checkout's
        const { DESTACK_CACHE_DIRECTORY: named } = environment.process.variables;
        if (named === "") {
            throw BuildError.of("CACHE_DIRECTORY_INVALID", "DESTACK_CACHE_DIRECTORY is empty");
        }
        const cache = named ?? join(root, CHECKOUT_CACHE);

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
