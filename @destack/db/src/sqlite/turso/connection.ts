import * as turso from "@tursodatabase/database";
import type { EmptyRelations } from "drizzle-orm/relations";
import type { DrizzleSQLiteConfig } from "drizzle-orm/sqlite-core/utils";
import type * as declaration from "../../declare/database.ts";
import type { Table } from "../../table/table.ts";
import { SqliteDatabase } from "../database.ts";
import { pollNotifier, type CommitNotifier } from "../../log/notifier.ts";

/**
 * How often to look for commits other processes made to a file while readers wait, in milliseconds.
 *
 * One poll reads the latest sequence in microseconds, so ten polls a second cost nothing and bound the delay.
 */
const COMMIT_POLL_MILLISECONDS = 100;

/** Open an embedded Turso database and bind its declared tables. */
export async function connect(
    connection: string | turso.Database,
    tables: declaration.Database | readonly Table[] = [],
    options: ConnectOptions = {},
): Promise<SqliteDatabase<turso.Database>> {
    // enable generated columns for connections created by Destack
    const client =
        typeof connection === "string"
            ? await turso.connect(connection, { experimental: ["generated_columns"] })
            : connection;

    try {
        // enable durable commits and foreign keys on connections opened here
        if (typeof connection === "string") {
            await client.exec("PRAGMA synchronous = FULL; PRAGMA foreign_keys = ON;");
        }

        // notice other writers' commits by polling the file unless a notifier is given
        const { notifier = pollNotifier(COMMIT_POLL_MILLISECONDS), ...drizzle } = options;

        return new SqliteDatabase(client, tables, notifier, drizzle);
    } catch (error) {
        // release only connections allocated by this call
        if (typeof connection === "string") {
            try {
                await client.close();
            } catch (cleanup) {
                throw new AggregateError([error, cleanup], "database binding and closure failed");
            }
        }

        throw error;
    }
}

/** How to bind a Turso database: Drizzle's settings and the notifications exchanged with other writers. */
export type ConnectOptions = Omit<DrizzleSQLiteConfig<EmptyRelations>, "relations"> & {
    /** The notifications exchanged with other writers, polling by default. */
    readonly notifier?: CommitNotifier;
};
