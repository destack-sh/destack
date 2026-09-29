import { TABLE, type Table } from "../table/table.ts";
import type { Column } from "../table/column.ts";
import type { ChangeDescription } from "../inspect/log.ts";
import { DatabaseError } from "../error/error.ts";
import { v7 } from "uuid";
import { literal, quote } from "../dialect/quote.ts";

/** The SQL name of the database's log. */
export const LOG = "__destack_log";

/** The SQL name of the highest compacted change sequence. */
export const LOG_HORIZON = "__destack_log_horizon";

/** The SQL name of the consumers' held positions. */
export const LOG_HOLD = "__destack_log_hold";

/** The SQL name of the log's epoch. */
export const LOG_EPOCH = "__destack_log_epoch";

/** The SQL name of the marker of a transaction writing derived rows. */
export const LOG_COPYING = "__destack_log_copying";

/** The SQL name of the open SQLite transaction's identity. */
export const LOG_TRANSACTION = "__destack_log_transaction";

/** The PostgreSQL notification channel of logged commits. */
export const LOG_CHANNEL = "destack_log";

/** Describe the columns a table's change triggers record, absent for unlogged tables. */
export function describeLog(table: Table): ChangeDescription | undefined {
    // skip unlogged tables
    const definition = table[TABLE];
    if (definition.retention === "none") {
        return undefined;
    }

    // file each change under the row's scope
    const scope = definition.columns.scope;
    if (scope === undefined || scope.definition.nullable) {
        throw new DatabaseError(
            "INVALID_MIGRATION",
            `logged table has no required scope column: ${definition.name}`,
        );
    }

    // record every column but binary and sensitive ones
    const columns = Object.values(definition.columns).map((column) => column.definition);
    const logged = table[TABLE].logged;
    const recorded = Object.values(logged).map((column) => column.definition);
    const unlogged = definition.key.find((property) => !Object.hasOwn(logged, property));
    if (unlogged !== undefined) {
        throw new DatabaseError(
            "INVALID_MIGRATION",
            `logged key column is binary or sensitive: ${definition.name}.${unlogged}`,
        );
    }

    return {
        table: definition.sqlName,
        retention: definition.retention,
        key: primaryKey(table).map((column) => column.definition.name),
        columns: recorded.map((column) => column.name),
        exact: recorded
            .filter((column) => column.kind === "bigint" || column.kind === "numeric")
            .map((column) => column.name),
        compared: columns.map((column) => column.name),
        scope: scope.definition.name,
    };
}

/** Read a logged table's primary key columns in key order. */
export function primaryKey(table: Table): readonly Column[] {
    // require a key
    const columns = table[TABLE].key.map((property) => table[TABLE].columns[property]!);
    if (columns.length === 0) {
        throw new DatabaseError(
            "INVALID_MIGRATION",
            `logged table has no primary key: ${table[TABLE].sqlName}; declare its changes as "none"`,
        );
    }

    return columns;
}

/** Create the log's epoch once and keep existing state. */
export function createEpoch(): readonly string[] {
    return [
        `CREATE TABLE IF NOT EXISTS ${quote(LOG_HOLD)} (
            name TEXT PRIMARY KEY,
            sequence BIGINT NOT NULL,
            expires_at BIGINT NOT NULL
        )`,
        `CREATE TABLE IF NOT EXISTS ${quote(LOG_COPYING)} (slot INTEGER PRIMARY KEY CHECK (slot = 1))`,
        `CREATE TABLE IF NOT EXISTS ${quote(LOG_EPOCH)} (
            slot INTEGER PRIMARY KEY CHECK (slot = 1),
            epoch TEXT NOT NULL
        )`,
        `INSERT INTO ${quote(LOG_EPOCH)} (slot, epoch) VALUES (1, ${literal(v7())})
            ON CONFLICT (slot) DO NOTHING`,
    ];
}

/** The log of one dialect: its tables and change triggers. */
export interface LogDialect {
    /** Create the log once per database. */
    create(): readonly string[];
    /** Generate the triggers recording one table's changes. */
    install(description: ChangeDescription): string[];
    /** Remove the triggers recording one table's changes. */
    remove(description: ChangeDescription): string[];
}
