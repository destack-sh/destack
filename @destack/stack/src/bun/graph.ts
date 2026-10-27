import { type FileSystem, FileSystemError } from "@destack/environment";
import { join } from "node:path";
import { Database } from "bun:sqlite";
import { connectBunSqlite } from "@destack/db/bun";
import { type DatabaseConnection, type DatabaseEnvironment, TABLE } from "@destack/db";
import type { ObjectType } from "@destack/object";
import type { PackageStore } from "@destack/build/store";
import type { ExternalGraphStorage } from "../server/resource/retention.ts";
import { Digest, nth, Uuid, zip } from "@destack/schema";
import { catalog, declaration, GraphIndex, module, symbol } from "../object/graph.ts";

/** The most parameters SQLite binds to a statement: SQLITE_MAX_VARIABLE_NUMBER since 3.32. */
const SQLITE_PARAMETERS = 32_766;

/** The graph of each build as external objects. */
export class BunGraphStorage implements ExternalGraphStorage, AsyncDisposable {
    /** The graph's object types. */
    readonly objects: readonly ObjectType[] = [module, declaration, symbol, catalog];
    /** The database with the builds. */
    readonly #database: DatabaseConnection;
    /** The builds and code indexes the spaces store. */
    readonly #builds: Pick<PackageStore, "contains" | "contents" | "referrer" | "file">;
    /** The directory with one index file per manifest, which every build of it reads. */
    readonly #directory: string;
    /** The files the index files live in. */
    readonly #files: FileSystem;
    /** The opened indexes, by build. */
    readonly #opened = new Map<string, Promise<OpenedIndex>>();

    /** Index the builds a database records and their spaces store, in a directory of files. */
    constructor(
        database: DatabaseConnection,
        builds: Pick<PackageStore, "contains" | "contents" | "referrer" | "file">,
        directory: string,
        files: FileSystem,
    ) {
        // store the database, the builds and the directory of the index files
        this.#database = database;
        this.#builds = builds;
        this.#directory = directory;
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

    /** Delete and close the indexes of the manifests the build store no longer has. */
    async sweep(): Promise<void> {
        // close the swept builds' indexes except the failed opens
        const opened = [...this.#opened];
        const settled = await Promise.allSettled(opened.map(([, index]) => index));
        for (const [[scope], index] of zip(opened, settled)) {
            if (
                index.status === "fulfilled" &&
                !(await this.#builds.contains(index.value.manifest))
            ) {
                this.#opened.delete(scope);
                await index.value.database.close();
            }
        }

        // delete the index files and pending writes of each manifest the store dropped
        const entries = await this.#files.readDirectory(this.#directory).catch((error: unknown) => {
            // read no files of a directory no index wrote yet
            if (FileSystemError.is(error, "NOT_FOUND")) {
                return [];
            }

            throw error;
        });
        const names = entries.map((entry) => entry.name);
        for (const [prefix, kept] of Map.groupBy(names, (name) => nth(name.split("."), 0))) {
            // delete the files of a build the store no longer has, named by its manifest's digest
            const manifest = Digest.safeParse(prefix);
            if (!manifest.success || !(await this.#builds.contains(manifest.data))) {
                for (const file of kept) {
                    await this.#files.remove(join(this.#directory, file));
                }
            }
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
        const { environment } = this.#database.state;
        const file = join(this.#directory, `${manifest}.sqlite`);
        if (!(await this.#files.exists(file))) {
            await this.#files.mkdir(this.#directory, { recursive: true });
            await write(index, this.#builds, file, environment, this.#files);
        }

        // read it as the build's rows: in its scope, made when it finished
        const database = new Database(file, { readonly: true });
        for (const table of GraphIndex.tables) {
            const { sqlName } = table[TABLE];
            database.run(
                `CREATE TEMP VIEW "${sqlName}" AS ${index.select(table, `main."${sqlName}"`)}`,
            );
        }

        return {
            manifest,
            database: await connectBunSqlite(database, GraphIndex.tables, { environment }),
        };
    }
}

/** A build's opened index. */
interface OpenedIndex {
    /** The digest of the build's manifest. */
    readonly manifest: Digest;
    /** The read-only index database. */
    readonly database: DatabaseConnection & { readonly close: () => Promise<void> };
}

/** Write a manifest's objects into a new index file as the indexing build's, published whole. */
async function write(
    index: GraphIndex,
    builds: Pick<PackageStore, "contains" | "contents" | "referrer" | "file">,
    file: string,
    environment: DatabaseEnvironment,
    files: FileSystem,
): Promise<void> {
    // derive the build's objects
    const rows = await index.rows(builds);

    // write a private file in the rollback journal mode read-only readers open
    const pending = `${file}.${Uuid.v4(environment.random)}`;
    const database = await connectBunSqlite(new Database(pending), GraphIndex.tables, {
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

    // publish the file under its final name
    await files.rename(pending, file);
}
