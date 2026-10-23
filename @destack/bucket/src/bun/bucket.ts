import { constants } from "node:fs";
import { copyFile, mkdir, stat } from "node:fs/promises";
import type { Clock } from "@destack/environment";
import { dirname, join, resolve } from "node:path";
import { connectBunSqlite } from "@destack/db/bun";
import type { SqliteDatabase } from "@destack/db/sqlite";
import { FileLock } from "@destack/fs";
import { BucketKey } from "../bucket/index.ts";
import { BucketError } from "../error/index.ts";
import { CatalogueBucket } from "../catalogue/bucket.ts";
import { CatalogueStorage } from "../catalogue/storage.ts";
import { catalogueDatabase } from "../catalogue/stack/index.ts";
import { syncDirectory } from "./directory.ts";
import { LocalBlobStore } from "./store.ts";

/** A bucket in a local directory, which processes share: a SQLite catalogue beside blob files. */
export class LocalBucket extends CatalogueBucket {
    /** The open catalogue and its blobs. */
    readonly #storage: CatalogueStorage;
    /** The blob files. */
    readonly #blobs: LocalBlobStore;

    /** Serve a catalogue over its blob files. */
    private constructor(storage: CatalogueStorage, blobs: LocalBlobStore) {
        super(storage);
        this.#storage = storage;
        this.#blobs = blobs;
    }

    /** Open a bucket in a directory, or create it in its scope, beside other processes' opens. */
    static async open(
        directory: string,
        scope: string | undefined,
        clock: Clock,
    ): Promise<LocalBucket> {
        // refuse creating a bucket without the scope it belongs to
        directory = resolve(directory);
        const catalogue = join(directory, "bucket.db");
        const isNew = await isMissing(catalogue);
        if (isNew && scope === undefined) {
            throw BucketError.of("NO_SUCH_BUCKET", `no bucket at ${directory}`);
        }

        // create the blob directory before publishing any catalogue entries
        const contents = join(directory, "files");
        await createDurably(contents);
        const blobs = await LocalBlobStore.open(contents);
        const lock = await FileLock.acquireShared(join(directory, "bucket.lock"));

        // prepare the catalogue while no other opener migrates it, then serve it through storage
        const database = await migrating(directory, async () => {
            const isCreated = await isMissing(catalogue);

            return await prepare(directory, catalogue, isCreated ? scope : undefined, lock);
        });

        return new LocalBucket(openedStorage(catalogue, database, blobs, lock, clock), blobs);
    }

    /** Clone a plain file's bytes to a path, copying them where the file system cannot clone. */
    async clone(key: string, destination: string): Promise<void> {
        // read the current plain single-part file's blob
        BucketKey.check(key);
        const blob = await this.#storage.exclusive(async () => {
            // refuse a missing file, then one with several or encrypted segments
            const entry = await this.#storage.entry(key);
            if (!entry) {
                throw BucketError.of("NO_SUCH_KEY", `no file at ${key}`);
            }
            const [segment, ...rest] = await this.#storage.segments(entry.version);
            if (segment === undefined || rest.length > 0 || segment.nonce !== null) {
                throw BucketError.of(
                    "FILE_NOT_CLONABLE",
                    `cloning needs a plain single-part file: ${key}`,
                );
            }

            // retain the blob against collection while cloning
            this.#storage.retain(segment.blob);

            return segment.blob;
        });

        // clone the blob file, then release it
        try {
            await copyFile(this.#blobs.path(blob), destination, constants.COPYFILE_FICLONE);
        } finally {
            this.#storage.release(blob);
        }
    }
}

/** Migrate a catalogue under the bucket's migration lock, as openers migrate one at a time. */
async function migrating<Result>(
    directory: string,
    migration: () => Promise<Result>,
): Promise<Result> {
    const lock = await FileLock.acquire(join(directory, "migrate.lock"));
    try {
        return await migration();
    } finally {
        await lock.close();
    }
}

/** Decide whether a path is missing, rethrowing every other failure. */
async function isMissing(path: string): Promise<boolean> {
    return await stat(path).then(
        () => false,
        (error: NodeJS.ErrnoException) => {
            if (error.code !== "ENOENT") {
                throw error;
            }

            return true;
        },
    );
}

/** Create a directory and persist each newly created entry through its existing parent. */
async function createDurably(directory: string): Promise<void> {
    // create the directory and its missing parents
    const created = await mkdir(directory, { recursive: true, mode: 0o700 });
    if (created === undefined) {
        return;
    }

    // sync each created entry up to the first one
    const parent = dirname(created);
    let current = directory;
    while (true) {
        await syncDirectory(current);
        if (current === parent) {
            break;
        }
        current = dirname(current);
    }
}

/** Prepare a bucket's private catalogue through the shared database lifecycle. */
async function prepare(
    directory: string,
    catalogue: string,
    created: string | undefined,
    lock: FileLock,
): Promise<SqliteDatabase> {
    let database: SqliteDatabase | undefined;
    try {
        // migrate the catalogue, creating the log of a new bucket
        database = await connectBunSqlite(catalogue, catalogueDatabase);
        if (created !== undefined) {
            await database.log.create(created);
        }
        await database.migrate(catalogueDatabase.tables);

        // persist the initial catalogue and lock entries before accepting writes
        await syncDirectory(directory);

        return database;
    } catch (error) {
        return await closeFailed(error, database, lock);
    }
}

/** Close a catalogue and its lock after a failed preparation. */
async function closeFailed(
    error: unknown,
    database: SqliteDatabase | undefined,
    lock: FileLock,
): Promise<never> {
    // close the database and the lock, keeping each closure failure
    const failures = [error];
    try {
        await database?.close();
    } catch (cleanup) {
        failures.push(cleanup);
    }
    try {
        await lock.close();
    } catch (cleanup) {
        failures.push(cleanup);
    }

    // rethrow the failure with the closure failures beside it
    if (failures.length > 1) {
        throw new AggregateError(failures, "storage initialization and closure failed", {
            cause: error,
        });
    }
    throw error;
}

/** Serve a catalogue and its blobs. */
function openedStorage(
    catalogue: string,
    database: SqliteDatabase,
    blobs: LocalBlobStore,
    lock: FileLock,
    clock: Clock,
): CatalogueStorage {
    let opened = database;

    return new CatalogueStorage({
        get database() {
            return opened;
        },
        blobs,
        clock,
        migrate: async (beside) => {
            // migrate while no other opener migrates, then reopen the connection declaring every table
            const tables = [...catalogueDatabase.tables, ...beside];
            await migrating(dirname(catalogue), () => opened.migrate(tables));
            await opened.close();
            opened = await connectBunSqlite(catalogue, tables);
        },
        reclaim: async (deletion) => {
            // delete blobs only while this process alone holds the bucket, sharing it again after
            if (!(await lock.tryExclusive())) {
                return false;
            }
            try {
                await deletion();
            } finally {
                await lock.share();
            }

            return true;
        },
        close: async () => {
            // release the lock after the catalogue, even when closing the catalogue fails
            await using closing = new AsyncDisposableStack();
            closing.defer(() => lock.close());
            await opened.close();
        },
    });
}
