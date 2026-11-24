/// <reference types="bun" />
import { basename } from "node:path";
import { Database } from "bun:sqlite";
import type * as declaration from "../declare/database.ts";
import type { Table } from "../table/table.ts";
import { SqliteDatabase, type SqliteConnectOptions } from "../sqlite/database.ts";
import { BunClient } from "./client.ts";
import type { Model } from "../query/model.ts";

/** The wait for another process's write lock in milliseconds, far above a write's few. */
const BUSY_TIMEOUT_MILLISECONDS = 5000;

/** The bytes of a file read through memory maps: 128 MiB, as Rails 8 sets SQLite. */
const MMAP_BYTES = 134_217_728;

/** The bytes the WAL file truncates to after a checkpoint: 64 MiB, as Rails 8 sets SQLite. */
const JOURNAL_LIMIT_BYTES = 67_108_864;

/** The pages a connection caches: 2000, as Rails 8 sets SQLite, about 8 MB at 4 KiB pages. */
const CACHED_PAGES = 2000;

/** Open a SQLite database file or `:memory:`. */
export async function connectBunSqlite<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
>(
    connection: string | Database,
    tables: declaration.DatabaseSchema<Models> | readonly Table[],
    options: SqliteConnectOptions,
): Promise<SqliteDatabase<BunClient, Models>> {
    // open the file, or take the database given
    const database = typeof connection === "string" ? new Database(connection) : connection;

    try {
        // configure connections opened here
        if (typeof connection === "string") {
            for (const pragma of [
                `busy_timeout = ${BUSY_TIMEOUT_MILLISECONDS}`,
                "journal_mode = WAL",
                "synchronous = NORMAL",
                "foreign_keys = ON",
                `mmap_size = ${MMAP_BYTES}`,
                `journal_size_limit = ${JOURNAL_LIMIT_BYTES}`,
                `cache_size = ${CACHED_PAGES}`,
                "temp_store = MEMORY",
            ]) {
                database.run(`PRAGMA ${pragma}`);
            }
        }

        // read the database's name from its file, or memory for an in-memory one
        const name = database.filename === ":memory:" ? "memory" : basename(database.filename);

        // connect to the database's other connections on the given channels, or to none as the only writer
        return new SqliteDatabase(
            new BunClient(database, options.environment),
            tables,
            "embedded",
            "writer",
            { ...options, name },
        );
    } catch (error) {
        // release only connections opened by this call
        if (typeof connection === "string") {
            database.close();
        }

        throw error;
    }
}
