import { quote } from "../dialect/quote.ts";
import type { Session } from "../database/session.ts";
import { v7 } from "uuid";
import type { ConnectionState } from "../database/connection.ts";
import { LOG_TRANSACTION } from "./schema.ts";
import { relation } from "../table/namespace.ts";
import { FullTextIndex } from "../search/search.ts";

/** Identify a SQLite transaction to the change triggers, absent without a log. */
export async function openTransaction(
    session: Session,
    connection: ConnectionState,
): Promise<string | undefined> {
    // look for the log until a migration created it
    if (!connection.isLogged) {
        const found = await session.values(
            "SELECT name FROM sqlite_schema WHERE type = 'table' AND name = ?",
            [relation(LOG_TRANSACTION, connection.namespace)],
        );
        if (found.length === 0) {
            return undefined;
        }
        connection.isLogged = true;
    }
    const marker = quote(relation(LOG_TRANSACTION, connection.namespace));
    const id = v7();
    await session.run(`INSERT INTO ${marker} (slot, id) VALUES (1, ?)`, [id]);

    return id;
}

/** Keep the transaction's searched rows' full-text rows, then clear its identity before commit. */
export async function closeTransaction(
    session: Session,
    connection: ConnectionState,
): Promise<void> {
    // keep the full-text rows, then clear the identity
    await FullTextIndex.keep(session, connection.searched, connection.namespace);
    const marker = quote(relation(LOG_TRANSACTION, connection.namespace));
    await session.run(`DELETE FROM ${marker} WHERE slot = 1`, []);
}
