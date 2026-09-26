import { type SQL, sql } from "drizzle-orm";
import { v7 } from "uuid";
import type { ConnectionState } from "../database/connection.ts";
import { LOG_TRANSACTION } from "./trigger.ts";

/** A native SQLite transaction able to run statements. */
interface SQLiteTransaction {
    /** Execute one statement. */
    run(statement: SQL): Promise<unknown>;
    /** Read rows. */
    all<Row>(statement: SQL): Promise<Row[]>;
}

/** Identify a SQLite transaction to the change triggers, when the database has a log. */
export async function openTransaction(
    transaction: SQLiteTransaction,
    connection: ConnectionState,
): Promise<boolean> {
    // look for the log until a migration has created it
    if (!connection.isLogged) {
        const [found] = await transaction.all<{ name: string }>(
            sql`SELECT name FROM sqlite_schema WHERE type = 'table' AND name = ${LOG_TRANSACTION}`,
        );
        if (!found) {
            return false;
        }
        connection.isLogged = true;
    }
    await transaction.run(
        sql`INSERT INTO ${sql.identifier(LOG_TRANSACTION)} (slot, id) VALUES (1, ${v7()})`,
    );

    return true;
}

/** Clear the transaction identity before commit, leaving statements outside transactions unidentified. */
export async function closeTransaction(transaction: SQLiteTransaction): Promise<void> {
    await transaction.run(sql`DELETE FROM ${sql.identifier(LOG_TRANSACTION)} WHERE slot = 1`);
}
