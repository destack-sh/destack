import { dirname, join } from "node:path";
import { LocalBucket } from "@destack/bucket/bun";
import { locateWorkspace } from "@destack/check/workspace";
import type { Environment } from "@destack/environment";
import { Identities } from "../cache/index.ts";
import type { PackageFile } from "@destack/package/file";
import { PackageStore } from "./store.ts";
import { BuildError } from "../error/index.ts";
import type { BuildReader } from "@destack/package/manifest";
import { STAGING_DIRECTORY } from "../dependency/tree.ts";
import { Digest } from "@destack/schema";

/** The directory below a store's bucket with the dependency trees of its builds, by manifest. */
const TREES_DIRECTORY = "trees";

/** The prefix of a root a directory names, forgotten once the directory is gone. */
const DIRECTORY_ROOT_PREFIX = "directory:";

/** The file every process using a store's bucket locks shared. */
const BUCKET_LOCK = "bucket.lock";

/** The scope of a local store's bucket: this machine's builds, as `universe` is the universe's. */
const SCOPE = "builds";

/** The file a stores' directory locks while one process sweeps and opens its stores. */
const DIRECTORY_LOCK = "stores.lock";

/** How long a store keeps an unrooted build past its last use: a week, as Nx keeps its cache. */
const CACHE_MILLISECONDS = 7 * 24 * 60 * 60_000;

/** How often an idle open sweeps: once a day, so a build a week unused goes within eight days. */
const SWEEP_MILLISECONDS = 24 * 60 * 60_000;

/** The cache directory below a checkout's root, where find-cache-dir places tools' caches. */
const CHECKOUT_CACHE = join("node_modules", ".cache", "destack");

/** What a checkout's store has builds for: development tools, or test runs. */
export type LocalStoreUse = "development" | "test";

/** A store of builds on this machine's disk, one per toolchain, which processes share. */
export class LocalPackageStore
    extends PackageStore<Environment<"files" | "process" | "host">>
    implements AsyncDisposable
{
    /** The directory of the dependency trees its builds are materialized in, one per manifest. */
    readonly trees: string;
    /** The bucket that stores the builds. */
    readonly #bucket: LocalBucket;
    /** The builds this store used since it opened, each leased once. */
    readonly #used = new Set<Digest>();
    /** What disposal releases: the bucket the store opened, and a temporary store's directory. */
    readonly #disposables: AsyncDisposableStack;

    /** Store the store's bucket below its directory, and what disposal releases. */
    private constructor(
        bucket: LocalBucket,
        directory: string,
        environment: Environment<"files" | "process" | "host">,
        disposables: AsyncDisposableStack,
    ) {
        // store the bucket, its environment, its trees and its disposables
        super(bucket, environment);
        this.trees = join(directory, TREES_DIRECTORY);
        this.#bucket = bucket;
        this.#disposables = disposables;
    }

    /** Store the builds of a bucket its opener opened and closes, their trees below a directory. */
    static async of(
        bucket: LocalBucket,
        directory: string,
        environment: Environment<"files" | "process" | "host">,
    ): Promise<LocalPackageStore> {
        return await LocalPackageStore.#create(
            bucket,
            directory,
            environment,
            new AsyncDisposableStack(),
        );
    }

    /** Store a bucket's builds, creating the directory of their trees. */
    static async #create(
        bucket: LocalBucket,
        directory: string,
        environment: Environment<"files" | "process" | "host">,
        disposables: AsyncDisposableStack,
    ): Promise<LocalPackageStore> {
        const store = new LocalPackageStore(bucket, directory, environment, disposables);
        await environment.files.mkdir(store.trees, { recursive: true });

        return store;
    }

    /** Store a build, keeping it a week past this use. */
    override async put(...stored: Parameters<PackageStore["put"]>): Promise<Digest> {
        const digest = await super.put(...stored);
        await this.#use(digest);

        return digest;
    }

    /** Open a stored build, keeping it a week past this use, as a cache keeps what it serves. */
    override async contents(digest: Digest): Promise<BuildReader> {
        await this.#use(digest);

        return super.contents(digest);
    }

    /** Lease a build once per open store, sparing it and its files from sweeps a week on. */
    async #use(digest: Digest): Promise<void> {
        if (!this.#used.has(digest)) {
            this.#used.add(digest);
            await this.lease(digest, new Date(this.environment.clock.now() + CACHE_MILLISECONDS));
        }
    }

    /** Root some builds by a directory, until the directory is gone. */
    async rootDirectory(directory: string, manifests: ReadonlySet<string>): Promise<void> {
        await this.root(`${DIRECTORY_ROOT_PREFIX}${directory}`, manifests);
    }

    /** Clone a stored file to a path, creating its folder, copying where cloning is unsupported. */
    async place(file: PackageFile, destination: string): Promise<void> {
        await this.environment.files.mkdir(dirname(destination), { recursive: true });
        await this.#bucket.clone(PackageStore.fileKey(file.digest), destination);
    }

    /** Forget the roots of deleted directories, then sweep what no root retains, as of now. */
    async #sweepIdle(now: Date): Promise<void> {
        // forget each root named by a directory that no longer exists
        for (const name of (await this.roots()).keys()) {
            const directory = name.startsWith(DIRECTORY_ROOT_PREFIX)
                ? name.slice(DIRECTORY_ROOT_PREFIX.length)
                : undefined;
            if (directory !== undefined && !(await this.environment.files.exists(directory))) {
                await this.root(name, new Set());
            }
        }

        // delete what crashed materializations left staged
        const staging = join(this.trees, STAGING_DIRECTORY);
        await this.environment.files.remove(staging, { recursive: true, force: true });

        // sweep unrooted builds older than a week, once a day
        await this.sweepIfDue(
            new Date(now.getTime() - CACHE_MILLISECONDS),
            now,
            SWEEP_MILLISECONDS,
        );
    }

    /** Delete what retention drops, and the dependency trees of the builds it deleted. */
    override async sweep(...options: Parameters<PackageStore["sweep"]>): Promise<void> {
        // sweep the bucket, then delete each tree whose build the store no longer has
        await super.sweep(...options);
        const { files } = this.environment;
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

    /** Open the toolchain's store below a directory, sweeping idle stores of every toolchain. */
    static async open(
        directory: string,
        environment: Environment<"files" | "process" | "host">,
    ): Promise<LocalPackageStore> {
        // lock the directory while sweeping and opening, so no process opens a store being deleted
        const { clock, files } = environment;
        const toolchain = await Identities.of({}, environment).toolchain();
        await files.mkdir(directory, { recursive: true });
        const held = await files.lock(join(directory, DIRECTORY_LOCK), "exclusive");
        try {
            // delete each other toolchain's store no process has open, leaving files such as .DS_Store
            const entries = await files.readDirectory(directory);
            const stores = entries
                .filter((entry) => entry.kind === "directory" && entry.name !== toolchain)
                .map((entry) => entry.name);
            for (const entry of stores) {
                const lock = await files.tryLock(join(directory, entry, BUCKET_LOCK), "exclusive");
                if (lock !== undefined) {
                    await files.remove(join(directory, entry), { recursive: true });
                    await lock.close();
                }
            }

            // learn whether another process has the toolchain's store open, as openers wait here
            await files.mkdir(join(directory, toolchain), { recursive: true });
            const location = await files.realPath(join(directory, toolchain));
            const idle = await files.tryLock(join(location, BUCKET_LOCK), "exclusive");
            await idle?.close();

            // open the store, then sweep it while no other process builds into it
            const disposables = new AsyncDisposableStack();
            const bucket = disposables.use(await LocalBucket.open(location, SCOPE, environment));
            const store = await LocalPackageStore.#create(
                bucket,
                location,
                environment,
                disposables,
            );
            if (idle !== undefined) {
                await store.#sweepIdle(new Date(clock.now()));
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
        // open a bucket in a new directory, both released on disposal
        const { files } = environment;
        const temporary = await files.makeTemporaryDirectory("destack-store-");
        const directory = await files.realPath(temporary);
        const disposables = new AsyncDisposableStack();
        disposables.defer(() => files.remove(directory, { recursive: true }));
        const bucket = disposables.use(await LocalBucket.open(directory, SCOPE, environment));

        return await LocalPackageStore.#create(bucket, directory, environment, disposables);
    }

    /** Release the store's disposables: close the bucket it opened, removing a temporary one. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#disposables.disposeAsync();
    }
}
