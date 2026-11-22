import type { FileSystem } from "@destack/environment";
import { join } from "node:path";
import { Database } from "bun:sqlite";
import { connectBunSqlite } from "@destack/db/bun";
import { TABLE } from "@destack/db";
import type { DatabaseConnection, DatabaseEnvironment } from "@destack/db/database";
import type { ObjectType } from "@destack/object";
import type { ExternalStorage } from "@destack/object/server";
import { LocalBlobStorage } from "@destack/bucket/bun";
import type { PackageFile } from "@destack/package/file";
import { Digest, present } from "@destack/schema";
import type { BuildStore } from "@destack/build/store";
import { catalog, declaration, BuildGraph, module, symbol } from "@destack/stack/object";

/** The most parameters SQLite binds to a statement: SQLITE_MAX_VARIABLE_NUMBER since 3.32. */
const SQLITE_PARAMETERS = 32_766;

/** The file name of a build's graph database, which also names its cache key with the manifest. */
const GRAPH_PATH = "graph.sqlite";

/** The media type of a graph database: a SQLite database file, as IANA registers it. */
const SQLITE_MEDIA_TYPE = "application/vnd.sqlite3";

/** The graph of each build as external objects, its index a file the store caches per manifest. */
export class BunGraphStorage implements ExternalStorage, AsyncDisposable {
    /** The graph's object types. */
    readonly objects: readonly ObjectType[] = [module, declaration, symbol, catalog];
    /** The database with the builds. */
    readonly #database: DatabaseConnection;
    /** The builds and graphs the space and its machine store. */
    readonly #builds: Pick<BuildStore, "storeOf" | "contents" | "image" | "graph">;
    /** The files the indexes are written in before they are stored. */
    readonly #files: FileSystem;
    /** The opened indexes, by build. */
    readonly #opened = new Map<string, Promise<OpenedIndex>>();

    /** Index the builds a database records, caching each index in the store with the build. */
    constructor(
        database: DatabaseConnection,
        builds: Pick<BuildStore, "storeOf" | "contents" | "image" | "graph">,
        files: FileSystem,
    ) {
        // store the database, the builds and the files
        this.#database = database;
        this.#builds = builds;
        this.#files = files;
    }

    /** Open a build's graph, indexing its module files on first use. */
    async open(scope: string): Promise<DatabaseConnection> {
        // open the build's index once and forget a failed open
        let opened = this.#opened.get(scope);
        if (opened === undefined) {
            opened = this.#open(scope);
            this.#opened.set(scope, opened);
            opened.catch(() => this.#opened.delete(scope));
        }

        return (await opened).database;
    }

    /** Close the open indexes. */
    async [Symbol.asyncDispose](): Promise<void> {
        // close each index whose open succeeded
        const opened = [...this.#opened.values()];
        this.#opened.clear();
        for (const index of await Promise.allSettled(opened)) {
            if (index.status === "fulfilled") {
                await index.value.database.close();
            }
        }
    }

    /** Open a succeeded build's view of its manifest's index, writing the index on a cache miss. */
    async #open(scope: string): Promise<OpenedIndex> {
        // read the build's place in its manifest's index, and the store with the build
        const index = await BuildGraph.of(this.#database, scope, this.#builds);
        const { manifest } = index;
        const store = present(
            await this.#builds.storeOf(manifest),
            `the store of build '${manifest}'`,
        );
        const { storage } = store.blobs;
        if (!(storage instanceof LocalBlobStorage)) {
            throw new TypeError(`the store of build '${manifest}' keeps no local files`);
        }

        // take the index the store caches for the manifest, which every build of it shares
        const { environment } = this.#database.state;
        const key = await this.cacheKey(manifest);
        const cached = await store.cached(key);
        if (cached !== undefined && cached.kind !== "file") {
            throw new TypeError(`the cache entry of '${manifest}'s index is no file`);
        }
        const stored = cached?.file ?? (await this.#cache(index, store, key, environment));

        // read it as the build's rows: in its scope, made when it finished
        const database = new Database(storage.path(stored.digest), { readonly: true });
        for (const table of BuildGraph.tables) {
            const { sqlName } = table[TABLE];
            database.run(
                `CREATE TEMP VIEW "${sqlName}" AS ${index.select(table, `main."${sqlName}"`)}`,
            );
        }

        return {
            database: await connectBunSqlite(database, BuildGraph.tables, { environment }),
        };
    }

    /** Name the cache entry of a manifest's index. */
    async cacheKey(manifest: Digest): Promise<Digest> {
        const named = new TextEncoder().encode(`${GRAPH_PATH} ${manifest}`);

        return await Digest.of(named, this.#database.state.environment.crypto);
    }

    /** Write a manifest's index into a file, then store it under its cache key. */
    async #cache(
        index: BuildGraph,
        store: BuildStore,
        key: Digest,
        environment: DatabaseEnvironment,
    ): Promise<PackageFile> {
        // write the index in a private directory, removed once stored
        const directory = await this.#files.makeTemporaryDirectory("destack-graph-");
        try {
            const file = join(directory, GRAPH_PATH);
            await write(index, this.#builds, file, environment);

            // store the file and cache it, which retains it while used
            const bytes = await this.#files.readFile(file);
            const digest = await Digest.of(bytes, environment.crypto);
            await store.blobs.put([
                { digest, size: bytes.byteLength, open: async () => chunks(bytes) },
            ]);
            const indexed = {
                path: GRAPH_PATH,
                digest,
                size: bytes.byteLength,
                mediaType: SQLITE_MEDIA_TYPE,
            };
            await store.cache(key, { kind: "file", file: indexed });

            return indexed;
        } finally {
            await this.#files.remove(directory, { recursive: true });
        }
    }
}

/** A build's opened index. */
interface OpenedIndex {
    /** The read-only index database. */
    readonly database: DatabaseConnection & { readonly close: () => Promise<void> };
}

/** Write a manifest's objects into a new index file, in the rollback journal mode readers open. */
async function write(
    index: BuildGraph,
    builds: Pick<BuildStore, "contents" | "graph">,
    file: string,
    environment: DatabaseEnvironment,
): Promise<void> {
    // derive the build's objects
    const rows = await index.rows(builds, environment.crypto);

    // write them in one transaction
    const database = await connectBunSqlite(new Database(file), BuildGraph.tables, {
        environment,
    });
    try {
        await database.migrate(BuildGraph.tables);
        await database.transaction((transaction) =>
            BuildGraph.write(transaction, rows, SQLITE_PARAMETERS),
        );
    } finally {
        await database.close();
    }
}

/** Yield some bytes as one chunk. */
async function* chunks(bytes: Uint8Array): AsyncIterable<Uint8Array> {
    yield bytes;
}
