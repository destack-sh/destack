import type { FileSystem } from "@destack/environment";
import { join } from "node:path";
import { Database } from "bun:sqlite";
import { connectBunSqlite } from "@destack/db/bun";
import { type DatabaseConnection, type DatabaseEnvironment, TABLE } from "@destack/db";
import type { ObjectType } from "@destack/object";
import type { ExternalStorage } from "@destack/object/server";
import type { PackageStore } from "@destack/build/store";
import { LocalBlobStorage } from "@destack/bucket/bun";
import type { PackageFile } from "@destack/package/file";
import { Digest } from "@destack/schema";
import type { BuildStore } from "../server/installation/build.ts";
import { catalog, declaration, GraphIndex, module, symbol } from "../object/graph.ts";

/** The most parameters SQLite binds to a statement: SQLITE_MAX_VARIABLE_NUMBER since 3.32. */
const SQLITE_PARAMETERS = 32_766;

/** The path a build's graph index is attached to its manifest at. */
const GRAPH_PATH = "graph";

/** The media type of a graph index: a SQLite database file, as IANA registers it. */
const SQLITE_MEDIA_TYPE = "application/vnd.sqlite3";

/** The graph of each build as external objects, its index a file attached to the build. */
export class BunGraphStorage implements ExternalStorage, AsyncDisposable {
    /** The graph's object types. */
    readonly objects: readonly ObjectType[] = [module, declaration, symbol, catalog];
    /** The database with the builds. */
    readonly #database: DatabaseConnection;
    /** The builds and code indexes the space and its machine store. */
    readonly #builds: Pick<BuildStore, "store" | "contains" | "contents" | "referrer" | "file">;
    /** The files the indexes are written in before they are stored. */
    readonly #files: FileSystem;
    /** The opened indexes, by build. */
    readonly #opened = new Map<string, Promise<OpenedIndex>>();

    /** Index the builds a database records, attaching each index to its build where stored. */
    constructor(
        database: DatabaseConnection,
        builds: Pick<BuildStore, "store" | "contains" | "contents" | "referrer" | "file">,
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

    /** Open a succeeded build's view of its manifest's index, attaching the index when absent. */
    async #open(scope: string): Promise<OpenedIndex> {
        // read the build's place in its manifest's index, and the store with the build
        const index = await GraphIndex.of(this.#database, scope);
        const { manifest } = index;
        const store = await this.#builds.store(manifest);
        const { storage } = store.blobs;
        if (!(storage instanceof LocalBlobStorage)) {
            throw new TypeError(`the store of build ${manifest} keeps no local files`);
        }

        // attach the index once per manifest, which every build of it shares
        const { environment } = this.#database.state;
        const attached =
            (await store.referrer(manifest, GRAPH_PATH)) ??
            (await this.#attach(index, store, environment));

        // read it as the build's rows: in its scope, made when it finished
        const database = new Database(storage.path(attached.digest), { readonly: true });
        for (const table of GraphIndex.tables) {
            const { sqlName } = table[TABLE];
            database.run(
                `CREATE TEMP VIEW "${sqlName}" AS ${index.select(table, `main."${sqlName}"`)}`,
            );
        }

        return {
            database: await connectBunSqlite(database, GraphIndex.tables, { environment }),
        };
    }

    /** Write a manifest's index into a file, then store it attached to the build. */
    async #attach(
        index: GraphIndex,
        store: PackageStore,
        environment: DatabaseEnvironment,
    ): Promise<PackageFile> {
        // write the index in a private directory, removed once stored
        const directory = await this.#files.makeTemporaryDirectory("destack-graph-");
        try {
            const file = join(directory, "graph.sqlite");
            await write(index, this.#builds, file, environment);

            // store the file and attach it to the build, which retains it
            const bytes = await this.#files.readFile(file);
            const digest = await Digest.of(bytes, environment.crypto);
            await store.blobs.put(digest, async () => chunks(bytes));
            const attached = {
                path: GRAPH_PATH,
                digest,
                size: bytes.byteLength,
                mediaType: SQLITE_MEDIA_TYPE,
            };
            await store.attach(index.manifest, attached);

            return attached;
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
    index: GraphIndex,
    builds: Pick<PackageStore, "contains" | "contents" | "referrer" | "file">,
    file: string,
    environment: DatabaseEnvironment,
): Promise<void> {
    // derive the build's objects
    const rows = await index.rows(builds);

    // write them in one transaction
    const database = await connectBunSqlite(new Database(file), GraphIndex.tables, {
        environment,
    });
    try {
        await database.migrate(GraphIndex.tables);
        await database.transaction((transaction) =>
            GraphIndex.write(transaction, rows, SQLITE_PARAMETERS),
        );
    } finally {
        await database.close();
    }
}

/** Yield some bytes as one chunk. */
async function* chunks(bytes: Uint8Array): AsyncIterable<Uint8Array> {
    yield bytes;
}
