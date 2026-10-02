import { quote } from "../dialect/quote.ts";
import type { Session } from "../database/session.ts";
import { v7 } from "uuid";
import type { ConnectionState } from "../database/connection.ts";
import { LOG_TRANSACTION } from "./schema.ts";

/** Identify a SQLite transaction to the change triggers. */
export async function openTransaction(
    session: Session,
    connection: ConnectionState,
): Promise<boolean> {
    // look for the log until a migration created it
    if (!connection.isLogged) {
        const found = await session.values(
            "SELECT name FROM sqlite_schema WHERE type = 'table' AND name = ?",
            [LOG_TRANSACTION],
        );
        if (found.length === 0) {
            return false;
        }
        connection.isLogged = true;
    }
    await session.run(`INSERT INTO ${quote(LOG_TRANSACTION)} (slot, id) VALUES (1, ?)`, [v7()]);

    return true;
}

/** Clear the transaction identity before commit. */
export async function closeTransaction(session: Session): Promise<void> {
    await session.run(`DELETE FROM ${quote(LOG_TRANSACTION)} WHERE slot = 1`, []);
}
