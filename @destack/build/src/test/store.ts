import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { LocalBlobStorage } from "@destack/bucket/bun";
import { BlobStore } from "@destack/db";
import { type BlobStorage, MemoryBlobStorage } from "@destack/resource";
import { connectBunSqlite } from "@destack/db/bun";
import type { SqliteDatabase } from "@destack/db/sqlite";
import type { Environment } from "@destack/environment";
import { testHost } from "@destack/test/host";
import { LocalPackageStore } from "../store/local.ts";
import { PackageStore } from "../store/store.ts";
import { packageStoreDatabase } from "../store/table.ts";

/** Open the store of builds tests share across runs, the host's, or a world's view of it. */
export async function testStore(
    world?: Environment<"files" | "process" | "host">,
): Promise<LocalPackageStore> {
    // open and maintain the shared store on the host, as one piece of host work in a world
    const here = fileURLToPath(new URL(".", import.meta.url));
    const host = LocalPackageStore.checkout(here, "test", testHost());
    if (world === undefined) {
        return await host;
    }

    // give the world a view of it through its own effects
    return await (await world.scheduler.track(host, "host")).view(world);
}

/** A package store over a test's blobs and index, in memory or below a directory. */
export class TestPackageStore extends PackageStore implements AsyncDisposable {
    /** The database with the index. */
    readonly #database: SqliteDatabase;

    /** Store builds in blobs indexed in a database the store closes. */
    private constructor(storage: BlobStorage, database: SqliteDatabase, environment: Environment) {
        super({ blobs: new BlobStore({ storage, database, environment }), environment });
        this.#database = database;
    }

    /** Open an empty store in memory. */
    static async create(environment: Environment): Promise<TestPackageStore> {
        return new TestPackageStore(
            new MemoryBlobStorage(environment.crypto),
            await TestPackageStore.#migrated(":memory:", environment),
            environment,
        );
    }

    /** Open the store below a directory, created once, as a template copies it. */
    static async open(
        directory: string,
        environment: Environment<"files">,
    ): Promise<TestPackageStore> {
        const storage = await LocalBlobStorage.open(join(directory, "files"), environment);
        const database = await TestPackageStore.#migrated(join(directory, "store.db"), environment);

        return new TestPackageStore(storage, database, environment);
    }

    /** Close the database. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#database.close();
    }

    /** Connect to an index database, migrated to the store's tables. */
    static async #migrated(file: string, environment: Environment): Promise<SqliteDatabase> {
        const database = await connectBunSqlite(file, packageStoreDatabase, { environment });
        await database.migrate(packageStoreDatabase.tables);

        return database;
    }
}
