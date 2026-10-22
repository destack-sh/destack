import { exists, mkdir, mkdtemp, readdir, realpath, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { LocalBucket } from "@destack/bucket/bun";
import { locateWorkspace } from "@destack/check/workspace";
import { Clock } from "@destack/environment";
import { FileLock } from "@destack/fs";
import { Identities, identifyToolchain } from "../cache/index.ts";
import type { PackageFile } from "@destack/package/file";
import { PackageStore } from "./store.ts";
import { STAGING_DIRECTORY } from "../dependency/tree.ts";
import { Digest } from "@destack/schema";

/** The directory below a store's bucket keeping the dependency trees of its builds, by manifest. */
const TREES_DIRECTORY = "trees";

/** The prefix of a root a directory names, forgotten once the directory is gone. */
const DIRECTORY_ROOT_PREFIX = "directory:";

/** The file every process holding a store's bucket locks shared. */
const BUCKET_LOCK = "bucket.lock";

/** The scope of a local store's bucket: this machine's builds, as `universe` is the universe's. */
const SCOPE = "builds";

/** The file a stores' directory locks while one process sweeps and opens its stores. */
const DIRECTORY_LOCK = "stores.lock";

/** The directory below a checkout's root keeping each use's builds, ignored as node_modules. */
const CHECKOUT_DIRECTORIES: Readonly<Record<LocalStoreUse, string>> = {
    development: join("node_modules", ".destack", "store"),
    test: join("node_modules", ".destack", "test-store"),
};

/** What a checkout's store keeps builds for: development tools, or test runs. */
export type LocalStoreUse = "development" | "test";

/** A store of builds on this machine's disk, one per toolchain, which processes share. */
export class LocalPackageStore extends PackageStore implements AsyncDisposable {
    /** The directory of the dependency trees its builds are materialized in, one per manifest. */
    readonly trees: string;
    /** The bucket keeping the builds. */
    readonly #bucket: LocalBucket;
    /** What disposal releases: the bucket the store opened, and a temporary store's directory. */
    readonly #owned: AsyncDisposableStack;

    /** Keep the store's bucket below its directory, and what disposal releases. */
    private constructor(bucket: LocalBucket, directory: string, owned: AsyncDisposableStack) {
        // keep the bucket, the trees below its directory and what the store owns
        super(bucket);
        this.trees = join(directory, TREES_DIRECTORY);
        this.#bucket = bucket;
        this.#owned = owned;
    }

    /** Keep the builds of a bucket its owner opened and closes, their trees below a directory. */
    static async of(bucket: LocalBucket, directory: string): Promise<LocalPackageStore> {
        return await LocalPackageStore.#create(bucket, directory, new AsyncDisposableStack());
    }

    /** Keep a bucket's builds, creating the directory of their trees. */
    static async #create(
        bucket: LocalBucket,
        directory: string,
        owned: AsyncDisposableStack,
    ): Promise<LocalPackageStore> {
        const store = new LocalPackageStore(bucket, directory, owned);
        await mkdir(store.trees, { recursive: true });

        return store;
    }

    /** Root some builds by a directory, until the directory is gone. */
    async rootDirectory(directory: string, manifests: ReadonlySet<string>): Promise<void> {
        await this.root(`${DIRECTORY_ROOT_PREFIX}${directory}`, manifests);
    }

    /** Clone a stored file to a path, creating its folder, copying where cloning is unsupported. */
    async place(file: PackageFile, destination: string): Promise<void> {
        await mkdir(dirname(destination), { recursive: true });
        await this.#bucket.clone(PackageStore.fileKey(file.digest), destination);
    }

    /** Forget the roots of deleted directories, then sweep what no root retains, opened now. */
    async #sweepIdle(): Promise<void> {
        // forget each root named by a directory that no longer exists
        for (const name of (await this.roots()).keys()) {
            const directory = name.startsWith(DIRECTORY_ROOT_PREFIX)
                ? name.slice(DIRECTORY_ROOT_PREFIX.length)
                : undefined;
            if (directory !== undefined && !(await exists(directory))) {
                await this.root(name, new Set());
            }
        }

        // delete what crashed materializations left staged
        await rm(join(this.trees, STAGING_DIRECTORY), { recursive: true, force: true });

        // mark what no root retains, deleting what an earlier open marked
        const now = new Date();
        await this.sweepIfDue(now, now, 0);
    }

    /** Delete what retention drops, and the dependency trees of the builds it deleted. */
    override async sweep(...options: Parameters<PackageStore["sweep"]>): Promise<void> {
        // sweep the bucket, then delete each tree whose build the store no longer keeps
        await super.sweep(...options);
        const entries = await readdir(this.trees, { withFileTypes: true });
        const trees = entries
            .filter((entry) => entry.isDirectory() && entry.name !== STAGING_DIRECTORY)
            .map((entry) => entry.name);
        for (const tree of trees) {
            const [manifest] = tree.split("+");
            if (!(await this.contains(Digest.parse(manifest)))) {
                await rm(join(this.trees, tree), { recursive: true, force: true });
            }
        }
    }

    /** Open the toolchain's store below a directory, sweeping idle stores of every toolchain. */
    static async open(directory: string): Promise<LocalPackageStore> {
        // hold the directory while sweeping and opening, so no process opens a store being deleted
        const toolchain = await identifyToolchain(Identities.of({}));
        await mkdir(directory, { recursive: true });
        const held = await FileLock.acquire(join(directory, DIRECTORY_LOCK));
        try {
            // delete each other toolchain's store no process keeps open, leaving files such as .DS_Store
            const entries = await readdir(directory, { withFileTypes: true });
            const stores = entries
                .filter((entry) => entry.isDirectory() && entry.name !== toolchain)
                .map((entry) => entry.name);
            for (const entry of stores) {
                const lock = await FileLock.tryAcquire(join(directory, entry, BUCKET_LOCK));
                if (lock !== undefined) {
                    await rm(join(directory, entry), { recursive: true });
                    await lock.close();
                }
            }

            // learn whether another process has the toolchain's store open, as openers wait here
            await mkdir(join(directory, toolchain), { recursive: true });
            const location = await realpath(join(directory, toolchain));
            const idle = await FileLock.tryAcquire(join(location, BUCKET_LOCK));
            await idle?.close();

            // open the store, then sweep it while no other process builds into it
            const owned = new AsyncDisposableStack();
            const bucket = owned.use(await LocalBucket.open(location, SCOPE, Clock.system));
            const store = await LocalPackageStore.#create(bucket, location, owned);
            if (idle !== undefined) {
                await store.#sweepIdle();
            }

            return store;
        } finally {
            await held.close();
        }
    }

    /** Open the store of the checkout enclosing a directory that keeps builds for one use. */
    static async checkout(directory: string, use: LocalStoreUse): Promise<LocalPackageStore> {
        // find the workspace root past a link to its lockfile
        const located = await locateWorkspace(directory);
        const root = dirname(await realpath(join(located, "bun.lock")));

        return await LocalPackageStore.open(join(root, CHECKOUT_DIRECTORIES[use]));
    }

    /** Open a store of one run's own builds, removed on disposal. */
    static async temporary(): Promise<LocalPackageStore> {
        // open a bucket in a new directory, both released on disposal
        const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-store-")));
        const owned = new AsyncDisposableStack();
        owned.defer(() => rm(directory, { recursive: true }));
        const bucket = owned.use(await LocalBucket.open(directory, SCOPE, Clock.system));

        return await LocalPackageStore.#create(bucket, directory, owned);
    }

    /** Release what the store owns: close the bucket it opened, removing a temporary one. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#owned.disposeAsync();
    }
}
