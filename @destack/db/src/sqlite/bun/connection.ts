/// <reference types="bun" />
import { Database } from "bun:sqlite";
import type { EmptyRelations } from "drizzle-orm/relations";
import type { DrizzleSQLiteConfig } from "drizzle-orm/sqlite-core/utils";
import type * as declaration from "../../declare/database.ts";
import type { Table } from "../../table/table.ts";
import { SqliteDatabase } from "../database.ts";
import { BunClient } from "./client.ts";
import type { OpenChannel } from "../../channel/channel.ts";

/**
 * The wait for another process's write lock, in milliseconds.
 *
 * A write commits within milliseconds, and only a stuck writer exceeds five seconds.
 */
const BUSY_TIMEOUT_MILLISECONDS = 5000;

/** Open a SQLite database file or `:memory:`. */
export async function connect(
    connection: string | Database,
    tables: declaration.Database | readonly Table[] = [],
    options: ConnectOptions = {},
): Promise<SqliteDatabase<BunClient>> {
    // open the file, or take the database given
    const database = typeof connection === "string" ? new Database(connection) : connection;

    try {
        // configure connections opened here
        if (typeof connection === "string") {
            for (const pragma of [
                "journal_mode = WAL",
                "synchronous = FULL",
                "foreign_keys = ON",
                `busy_timeout = ${BUSY_TIMEOUT_MILLISECONDS}`,
            ]) {
                database.run(`PRAGMA ${pragma}`);
            }
        }

        // reach the database's other connections on the given channels, or none as the sole writer
        const { openChannel, ...drizzle } = options;

        return new SqliteDatabase(
            new BunClient(database),
            tables,
            "embedded",
            openChannel,
            drizzle,
        );
    } catch (error) {
        // release only connections opened by this call
        if (typeof connection === "string") {
            database.close();
        }

        throw error;
    }
}

/** The options of a SQLite connection. */
export type ConnectOptions = Omit<DrizzleSQLiteConfig<EmptyRelations>, "relations"> & {
    /** Open a channel of a name to the database's other connections, absent for a sole writer. */
    readonly openChannel?: OpenChannel;
};
