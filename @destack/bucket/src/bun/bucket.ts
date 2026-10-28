import { dirname, isAbsolute, join } from "node:path";
import { connectBunSqlite } from "@destack/db/bun";
import type { SqliteDatabase } from "@destack/db/sqlite";
import type { Environment, FileLock, FileSystem } from "@destack/environment";
import { BucketKey } from "../bucket/index.ts";
import { BucketError } from "../error/index.ts";
import { CatalogueBucket } from "../catalogue/bucket.ts";
import { CatalogueStorage } from "../catalogue/storage.ts";
import { catalogueDatabase } from "../catalogue/stack/index.ts";
import { syncDirectory } from "./directory.ts";
import { LocalBlobStorage } from "./store.ts";

/** The effects a local bucket runs on: its files, and its catalogue's clock, random and network. */
export type LocalBucketEnvironment = Environment<"files">;

/** A bucket in a local directory, which processes share: a SQLite catalogue beside blob files. */
export class LocalBucket extends CatalogueBucket {
    /** The open catalogue and its blobs. */
    readonly #storage: CatalogueStorage;
    /** The blob files. */
    readonly #blobs: LocalBlobStorage;

    /** Serve a catalogue over its blob files. */
    private constructor(storage: CatalogueStorage, blobs: LocalBlobStorage) {
        super(storage);
        this.#storage = storage;
        this.#blobs = blobs;
    }

    /** Open a bucket in a directory, or create it in its scope, beside other processes' opens. */
    static async open(
        directory: string,
        scope: string | undefined,
        environment: LocalBucketEnvironment,
    ): Promise<LocalBucket> {
        // refuse a relative directory, and creating a bucket without the scope it belongs to
        const { files } = environment;
        if (!isAbsolute(directory)) {
            throw BucketError.of(
                "INVALID_ARGUMENT",
                `a bucket directory is absolute: ${directory}`,
            );
        }
        const catalogue = join(directory, "bucket.db");
        const isNew = !(await files.exists(catalogue));
        if (isNew && scope === undefined) {
            throw BucketError.of("NO_SUCH_BUCKET", `no bucket at ${directory}`);
        }

        // create the blob directory before publishing any catalogue entries, and share the bucket's lock until the open fails or the storage takes it
        const blobs = await LocalBlobStorage.open(join(directory, "files"), environment);
        await using opening = new AsyncDisposableStack();
        const lock = opening.use(await files.lock(join(directory, "bucket.lock"), "shared"));

        // prepare the catalogue while no other opener migrates it, then serve it through storage
        const database = await migrating(directory, files, async () => {
            const isCreated = !(await files.exists(catalogue));

            return await prepare(directory, catalogue, isCreated ? scope : undefined, environment);
        });

        // serve the prepared catalogue through storage, which now holds the lock
        const storage = openedStorage(catalogue, database, blobs, lock, environment);
        opening.move();

        return new LocalBucket(storage, blobs);
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
            await this.#blobs.copy(blob, destination);
        } finally {
            this.#storage.release(blob);
        }
    }
}

/** Migrate a catalogue under the bucket's migration lock, as openers migrate one at a time. */
async function migrating<Result>(
    directory: string,
    files: FileSystem,
    migration: () => Promise<Result>,
): Promise<Result> {
    await using _lock = await files.lock(join(directory, "migrate.lock"), "exclusive");

    return await migration();
}

/** Prepare a bucket's private catalogue, closing it on failure. */
async function prepare(
    directory: string,
    catalogue: string,
    created: string | undefined,
    environment: LocalBucketEnvironment,
): Promise<SqliteDatabase> {
    // migrate the catalogue, creating the log of a new bucket
    await using preparing = new AsyncDisposableStack();
    const database = await connectBunSqlite(catalogue, catalogueDatabase, { environment });
    preparing.defer(() => database.close());
    if (created !== undefined) {
        await database.log.create(created);
    }
    await database.migrate(catalogueDatabase.tables);

    // persist the initial catalogue and lock entries before accepting writes
    await syncDirectory(directory, environment.files);
    preparing.move();

    return database;
}

/** Serve a catalogue and its blobs. */
function openedStorage(
    catalogue: string,
    database: SqliteDatabase,
    blobs: LocalBlobStorage,
    lock: FileLock,
    environment: LocalBucketEnvironment,
): CatalogueStorage {
    const { files } = environment;
    let opened = database;

    return new CatalogueStorage({
        get database() {
            return opened;
        },
        blobs,
        environment,
        migrate: async (beside) => {
            // migrate while no other opener migrates, then reopen the connection declaring every table
            const tables = [...catalogueDatabase.tables, ...beside];
            await migrating(dirname(catalogue), files, () => opened.migrate(tables));
            await opened.close();
            opened = await connectBunSqlite(catalogue, tables, { environment });
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
