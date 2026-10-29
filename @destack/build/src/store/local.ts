import { dirname, join } from "node:path";
import { LocalBlobStorage } from "@destack/bucket/bun";
import { BlobStore, blobLease, eq } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { locateWorkspace } from "@destack/check/workspace";
import type { Environment } from "@destack/environment";
import type { PackageFile } from "@destack/package/file";
import { PackageStore } from "./store.ts";
import { packageStoreDatabase } from "./table.ts";
import { BuildError } from "../error/index.ts";
import type { BuildReader } from "@destack/package/manifest";
import { STAGING_DIRECTORY } from "../dependency/tree.ts";
import { Digest } from "@destack/schema";

/** The directory below a store's directory with its blobs, by digest. */
const FILES_DIRECTORY = "files";

/** The database file of a store of its own, which indexes its blobs. */
const DATABASE_FILE = "store.db";

/** The directory below a store's directory with the dependency trees of its builds, by manifest. */
const TREES_DIRECTORY = "trees";

/** The prefix of a root a directory names, forgotten once the directory is gone. */
const DIRECTORY_ROOT_PREFIX = "directory:";

/** The file every process using a store locks shared. */
const STORE_LOCK = "store.lock";

/** The file a store's directory locks while one process opens and collects the store. */
const OPEN_LOCK = "open.lock";

/** How long a store retains an unrooted build past its last use: a week, as Nx keeps its cache. */
const CACHE_MILLISECONDS = 7 * 24 * 60 * 60_000;

/** How often a store records a build's use: daily, a seventh of its retention. */
const USE_MILLISECONDS = 24 * 60 * 60_000;

/** The cache directory below a checkout's root, where find-cache-dir places tools' caches. */
const CHECKOUT_CACHE = join("node_modules", ".cache", "destack");

/** What a checkout's store has builds for: development tools, or test runs. */
export type LocalStoreUse = "development" | "test";

/** The blobs, database, trees directory and environment of a store on this machine's disk. */
export interface LocalPackageStoreOptions {
    /** The blobs, kept in a directory, whose database declares the store's tables. */
    readonly blobs: BlobStore<LocalBlobStorage>;
    /** The directory with the dependency trees. */
    readonly directory: string;
    /** The environment the store runs in. */
    readonly environment: Environment<"files" | "process" | "host">;
}

/** A store of builds on this machine's disk, its blobs in a directory, which processes share. */
export class LocalPackageStore
    extends PackageStore<Environment<"files" | "process" | "host">, LocalBlobStorage>
    implements AsyncDisposable
{
    /** The directory of the dependency trees its builds are materialized in, one per manifest. */
    readonly trees: string;
    /** What disposal releases: the database the store opened, and a temporary store's directory. */
    readonly #disposables: AsyncDisposableStack;

    /** Store builds in a directory's blobs indexed in a database, and what disposal releases. */
    private constructor(options: LocalPackageStoreOptions, disposables: AsyncDisposableStack) {
        // store the blobs, the database, the trees and the disposables
        super(options);
        this.trees = join(options.directory, TREES_DIRECTORY);
        this.#disposables = disposables;
    }

    /** Store builds in blobs and an opened database, their trees below a directory. */
    static async of(options: LocalPackageStoreOptions): Promise<LocalPackageStore> {
        return await LocalPackageStore.#create(options, new AsyncDisposableStack());
    }

    /** Store a directory's builds, creating the directory of their trees. */
    static async #create(
        options: LocalPackageStoreOptions,
        disposables: AsyncDisposableStack,
    ): Promise<LocalPackageStore> {
        const store = new LocalPackageStore(options, disposables);
        await options.environment.files.mkdir(store.trees, { recursive: true });

        return store;
    }

    /** Open a store of its own in a directory: its blobs, and its database migrated. */
    static async #openDirectory(
        directory: string,
        environment: Environment<"files" | "process" | "host">,
        disposables: AsyncDisposableStack,
    ): Promise<LocalPackageStore> {
        // open the blobs, then the database, closed on disposal
        const storage = await LocalBlobStorage.open(join(directory, FILES_DIRECTORY), environment);
        const database = await connectBunSqlite(
            join(directory, DATABASE_FILE),
            packageStoreDatabase,
            { environment },
        );
        disposables.defer(() => database.close());
        await database.migrate(packageStoreDatabase.tables);
        const blobs = new BlobStore({ storage, database, environment });

        return await LocalPackageStore.#create({ blobs, directory, environment }, disposables);
    }

    /** Store a build, retaining it a week past this use. */
    override async put(...stored: Parameters<PackageStore["put"]>): Promise<Digest> {
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

    /** Clone a stored file to a path, creating its folder, copying where cloning is unsupported. */
    async place(file: PackageFile, destination: string): Promise<void> {
        await this.environment.files.mkdir(dirname(destination), { recursive: true });
        await this.blobs.storage.copy(file.digest, destination);
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

        // delete what crashed materializations left staged, and what interrupted blob writes left
        const { files } = this.environment;
        await files.remove(join(this.trees, STAGING_DIRECTORY), { recursive: true, force: true });
        await this.blobs.storage.clean();

        // unrecord the builds no root or lease retains, and delete the blobs they let go
        await this.collect();
        await this.blobs.collect();

        // delete the trees of the builds the store no longer has
        const entries = await files.readDirectory(this.trees);
        const trees = entries
            .filter((entry) => entry.kind === "directory" && entry.name !== STAGING_DIRECTORY)
            .map((entry) => entry.name);
        for (const tree of trees) {
            const [manifest] = tree.split("+");
            if (!(await this.contains(Digest.parse(manifest)))) {
                await files.remove(join(this.trees, tree), { recursive: true, force: true });
            }
        }
    }

    /** Open the store in a directory, collecting it first when no other process has it open. */
    static async open(
        directory: string,
        environment: Environment<"files" | "process" | "host">,
    ): Promise<LocalPackageStore> {
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
            const shared = await files.lock(join(location, STORE_LOCK), "shared");
            disposables.defer(() => shared.close());
            const store = await LocalPackageStore.#openDirectory(
                location,
                environment,
                disposables,
            );
            if (idle !== undefined) {
                await store.#collectIdle();
            }

            return store;
        } finally {
            await held.close();
        }
    }

    /** Open the cache store of the checkout enclosing a directory, with builds for one use. */
    static async checkout(
        directory: string,
        use: LocalStoreUse,
        environment: Environment<"files" | "process" | "host">,
    ): Promise<LocalPackageStore> {
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

        return await LocalPackageStore.open(join(cache, use), environment);
    }

    /** Open a store of one run's builds, removed on disposal. */
    static async temporary(
        environment: Environment<"files" | "process" | "host">,
    ): Promise<LocalPackageStore> {
        // open a store in a new directory, both released on disposal
        const { files } = environment;
        const temporary = await files.makeTemporaryDirectory("destack-store-");
        const directory = await files.realPath(temporary);
        const disposables = new AsyncDisposableStack();
        disposables.defer(() => files.remove(directory, { recursive: true }));

        return await LocalPackageStore.#openDirectory(directory, environment, disposables);
    }

    /** Close the database the store opened, removing a temporary store. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#disposables.disposeAsync();
    }
}
