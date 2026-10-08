import { mkdir, stat } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { connectBunSqlite } from "@destack/db/bun";
import type { SqliteDatabase } from "@destack/db/sqlite";
import { FileLock } from "@destack/fs";
import { BucketError } from "../error/index.ts";
import { CatalogueBucket } from "../catalogue/bucket.ts";
import { CatalogueStorage } from "../catalogue/storage.ts";
import { catalogueDatabase } from "../catalogue/stack/index.ts";
import { syncDirectory } from "./directory.ts";
import { LocalBlobStore } from "./store.ts";

/** A bucket kept in a local directory: a SQLite catalogue of files beside the files of their blobs. */
export class LocalBucket extends CatalogueBucket {
    /** Open a bucket in a directory, or create it in the scope it belongs to: a SQLite catalogue beside its blob files, locked to this host. */
    static async open(directory: string, scope?: string): Promise<LocalBucket> {
        // refuse creating a bucket without the scope it belongs to
        directory = resolve(directory);
        const catalogue = join(directory, "bucket.db");
        const isNew = await isMissing(catalogue);
        if (isNew && scope === undefined) {
            throw new BucketError("NO_SUCH_BUCKET", `no bucket at ${directory}`);
        }

        // create the blob directory before publishing any catalogue entries
        const contents = join(directory, "files");
        await createDurably(contents);
        const blobs = await LocalBlobStore.open(contents);
        const lock = await FileLock.acquire(join(directory, "bucket.lock"));

        // prepare the private catalogue, then serve it through storage
        const database = await prepare(directory, catalogue, isNew ? scope : undefined, lock);

        return new LocalBucket(openedStorage(catalogue, database, blobs, lock));
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

/** Prepare a bucket's private catalogue through the shared database lifecycle, creating its log in a new bucket's scope. */
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

/** Close a catalogue and its lock after a failed preparation, rethrowing the failure with any closure failures. */
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

/** Serve a catalogue and its blobs, reopening the catalogue over the tables beside it and releasing the lock after it. */
function openedStorage(
    catalogue: string,
    database: SqliteDatabase,
    blobs: LocalBlobStore,
    lock: FileLock,
): CatalogueStorage {
    let opened = database;

    return new CatalogueStorage({
        get database() {
            return opened;
        },
        blobs,
        migrate: async (beside) => {
            // migrate, then reopen the connection declaring every table
            const tables = [...catalogueDatabase.tables, ...beside];
            await opened.migrate(tables);
            await opened.close();
            opened = await connectBunSqlite(catalogue, tables);
        },
        close: async () => {
            await opened.close();
            await lock.close();
        },
    });
}
