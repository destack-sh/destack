import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { LocalBlobStorage } from "@destack/bucket/bun";
import { BlobStore } from "@destack/db";
import { type BlobStorage, MemoryBlobStorage } from "@destack/resource";
import { connectBunSqlite } from "@destack/db/bun";
import type { SqliteDatabase } from "@destack/db/sqlite";
import type { Environment } from "@destack/environment";
import { testHost } from "@destack/test/host";
import { LocalBuildStore } from "../store/local.ts";
import { BuildStore } from "../store/store.ts";

/** Open the store of builds tests share across runs, the host's, or a world's view of it. */
export async function testStore(
    world?: Environment<"files" | "process" | "host">,
): Promise<LocalBuildStore> {
    // open and maintain the shared store on the host, as one piece of host work in a world
    const here = fileURLToPath(new URL(".", import.meta.url));
    const open = () => LocalBuildStore.checkout(here, "test", testHost());
    if (world === undefined) {
        return await open();
    }

    // give the world a view of it through its own effects
    return await (await world.scheduler.track(open, "host")).view(world, world.scheduler);
}

/** A build store over a test's blobs and index, in memory or below a directory. */
export class TestBuildStore extends BuildStore implements AsyncDisposable {
    /** The database with the index. */
    readonly #database: SqliteDatabase;

    /** Store builds in blobs indexed in a database the store closes. */
    private constructor(storage: BlobStorage, database: SqliteDatabase, environment: Environment) {
        super({ blobs: new BlobStore({ storage, database, environment }), environment });
        this.#database = database;
    }

    /** Open an empty store in memory. */
    static async create(environment: Environment): Promise<TestBuildStore> {
        return new TestBuildStore(
            new MemoryBlobStorage(environment.crypto),
            await TestBuildStore.#migrated(":memory:", environment),
            environment,
        );
    }

    /** Open the store below a directory, created once, as a template copies it. */
    static async open(
        directory: string,
        environment: Environment<"files">,
    ): Promise<TestBuildStore> {
        const storage = await LocalBlobStorage.open(join(directory, "files"), environment, {
            synchronous: "off",
        });
        const database = await TestBuildStore.#migrated(join(directory, "store.db"), environment);

        return new TestBuildStore(storage, database, environment);
    }

    /** Close the database. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#database.close();
    }

    /** Connect to an index database, migrated to the store's tables. */
    static async #migrated(file: string, environment: Environment): Promise<SqliteDatabase> {
        const database = await connectBunSqlite(file, BuildStore.database, { environment });
        await database.migrate(BuildStore.database.tables);

        return database;
    }
}
