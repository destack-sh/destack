import { mkdir, readdir, rename, rm, stat } from "node:fs/promises";
import { join } from "node:path";
import { Database } from "bun:sqlite";
import { connectBunSqlite } from "@destack/db/bun";
import { type DatabaseConnection, TABLE } from "@destack/db";
import type { ObjectType } from "@destack/object";
import type { ExternalStorage } from "@destack/object/server";
import type { BuildReader } from "@destack/package/manifest";
import { type Identifier, zip } from "@destack/schema";
import { catalog, GraphIndex, module, symbol } from "../object/graph.ts";

/** The most parameters SQLite binds to a statement: SQLITE_MAX_VARIABLE_NUMBER since 3.32. */
const SQLITE_PARAMETERS = 32_766;

/** The graph of each build as external objects. */
export class BunGraphStorage implements ExternalStorage, AsyncDisposable {
    /** The graph's object types. */
    readonly objects: readonly ObjectType[] = [module, symbol, catalog];
    /** The database with the builds. */
    readonly #database: DatabaseConnection;
    /** Read a build a space stores by its manifest's digest. */
    readonly #builds: (spaceId: Identifier<"space">, manifest: string) => Promise<BuildReader>;
    /** The directory with one index file per manifest, which every build of it reads. */
    readonly #directory: string;
    /** The opened indexes, by build. */
    readonly #opened = new Map<string, Promise<OpenedIndex>>();

    /** Index the builds a database records and their spaces store, under a directory. */
    constructor(
        database: DatabaseConnection,
        builds: (spaceId: Identifier<"space">, manifest: string) => Promise<BuildReader>,
        directory: string,
    ) {
        this.#database = database;
        this.#builds = builds;
        this.#directory = directory;
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

    /** Delete the indexes of the manifests retention drops, closing their builds' databases. */
    async sweep(retained: ReadonlySet<string>): Promise<void> {
        // close the dropped builds' indexes except the failed opens
        const opened = [...this.#opened];
        const settled = await Promise.allSettled(opened.map(([, index]) => index));
        for (const [[scope], index] of zip(opened, settled)) {
            if (index.status === "fulfilled" && !retained.has(index.value.manifest)) {
                this.#opened.delete(scope);
                await index.value.database.close();
            }
        }

        // delete their files
        const files = await readdir(this.#directory).catch((error: unknown) => {
            if (isMissing(error)) {
                return [];
            }

            throw error;
        });
        for (const file of files.filter((name) => !retained.has(name.split(".")[0] ?? name))) {
            await rm(join(this.#directory, file), { force: true });
        }
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

    /** Open a succeeded build's view of its manifest's index, writing the index when absent. */
    async #open(scope: string): Promise<OpenedIndex> {
        // read the build's place in its manifest's index
        const index = await GraphIndex.of(this.#database, scope);
        const { manifest } = index;

        // write the index once per manifest, which every build of it shares
        const file = join(this.#directory, `${manifest}.sqlite`);
        if (!(await exists(file))) {
            await mkdir(this.#directory, { recursive: true });
            await write(index, await this.#builds(index.space, manifest), file);
        }

        // read it as the build's rows: in its scope, made when it finished
        const database = new Database(file, { readonly: true });
        for (const table of GraphIndex.tables) {
            const { sqlName } = table[TABLE];
            database.run(
                `CREATE TEMP VIEW "${sqlName}" AS ${index.select(table, `main."${sqlName}"`)}`,
            );
        }

        return { manifest, database: await connectBunSqlite(database, GraphIndex.tables) };
    }
}

/** A build's opened index. */
interface OpenedIndex {
    /** The digest of the build's manifest. */
    readonly manifest: string;
    /** The read-only index database. */
    readonly database: DatabaseConnection & { readonly close: () => Promise<void> };
}

/** Write a manifest's objects into a new index file as the indexing build's, published whole. */
async function write(index: GraphIndex, reader: BuildReader, file: string): Promise<void> {
    // derive the build's objects
    const rows = await index.rows(reader);

    // write a private file in the rollback journal mode read-only readers open
    const pending = `${file}.${crypto.randomUUID()}`;
    const database = await connectBunSqlite(new Database(pending), GraphIndex.tables);
    try {
        await database.migrate(GraphIndex.tables);
        await database.transaction((transaction) =>
            GraphIndex.write(transaction, rows, SQLITE_PARAMETERS),
        );
    } finally {
        await database.close();
    }

    // publish the file under its final name
    await rename(pending, file);
}

/** Report whether a file exists. */
async function exists(file: string): Promise<boolean> {
    try {
        await stat(file);

        return true;
    } catch (error) {
        if (isMissing(error)) {
            return false;
        }

        throw error;
    }
}

/** Report whether a file system failure is a missing file. */
function isMissing(error: unknown): boolean {
    return error instanceof Error && "code" in error && error.code === "ENOENT";
}
