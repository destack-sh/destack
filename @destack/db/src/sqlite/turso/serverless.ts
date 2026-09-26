import * as turso from "@tursodatabase/serverless";
import type { Table } from "../../table/table.ts";
import { SqliteDatabase } from "../database.ts";
import { pollNotifier } from "../../log/notifier.ts";
import type { ConnectOptions } from "./connection.ts";

/**
 * How often to look for commits other clients made to a hosted database while readers wait, in milliseconds.
 *
 * One poll is an HTTP round trip of tens of milliseconds and a billed request, so one a second bounds cost and delay.
 */
const COMMIT_POLL_MILLISECONDS = 1000;

/** Open a hosted Turso database and bind its declared tables. */
export async function connect(
    connection: turso.Config | turso.Connection,
    tables: readonly Table[] = [],
    options: ConnectOptions = {},
): Promise<SqliteDatabase<turso.Connection>> {
    const client = connection instanceof turso.Connection ? connection : turso.connect(connection);

    try {
        // notice other clients' commits by polling unless a notifier, such as the servers' channel, is given
        const { notifier = pollNotifier(COMMIT_POLL_MILLISECONDS), ...drizzle } = options;

        return new SqliteDatabase(client, tables, "networked", notifier, drizzle);
    } catch (error) {
        // release only connections allocated by this call
        if (!(connection instanceof turso.Connection)) {
            try {
                await client.close();
            } catch (cleanup) {
                throw new AggregateError([error, cleanup], "database binding and closure failed");
            }
        }

        throw error;
    }
}
