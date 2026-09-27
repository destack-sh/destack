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

/** The SQL name of the log's epoch, which a restore renews since sequences restart with it. */
export const LOG_EPOCH = "__destack_log_epoch";

/** The SQL name of the marker a transaction holds while it writes rows a source already derived. */
export const LOG_COPYING = "__destack_log_copying";

/** The SQL name of the open SQLite transaction's identity. */
export const LOG_TRANSACTION = "__destack_log_transaction";

/** The PostgreSQL notification channel announcing commits that changed the log. */
export const LOG_CHANNEL = "destack_log";

/** Describe the columns a table's change triggers record, absent for unlogged tables. */
export function describeLog(table: Table): ChangeDescription | undefined {
    // skip unlogged tables
    const definition = table[TABLE];
    if (definition.tier === "none") {
        return undefined;
    }

    // file every change under the scope the row lives in
    const scope = definition.columns.scope;
    if (scope === undefined || scope.definition.nullable) {
        throw new DatabaseError(
            "INVALID_MIGRATION",
            `logged table has no required scope column: ${definition.name}`,
        );
    }

    // record every column except binary and sensitive ones, which the key names rows without
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
        tier: definition.tier,
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
    // require a key, since log entries name rows by it
    const columns = table[TABLE].key.map((property) => table[TABLE].columns[property]!);
    if (columns.length === 0) {
        throw new DatabaseError(
            "INVALID_MIGRATION",
            `logged table has no primary key: ${table[TABLE].sqlName}; declare its changes as "none"`,
        );
    }

    return columns;
}

/** Create the log's epoch once, keeping the epoch an earlier creation minted, and the copying marker. */
export function createEpoch(): readonly string[] {
    return [
        `CREATE TABLE IF NOT EXISTS ${quote(LOG_COPYING)} (slot INTEGER PRIMARY KEY CHECK (slot = 1))`,
        `CREATE TABLE IF NOT EXISTS ${quote(LOG_EPOCH)} (
            slot INTEGER PRIMARY KEY CHECK (slot = 1),
            epoch TEXT NOT NULL
        )`,
        `INSERT INTO ${quote(LOG_EPOCH)} (slot, epoch) VALUES (1, ${literal(v7())})
            ON CONFLICT (slot) DO NOTHING`,
    ];
}

/** The log of one dialect: its own tables, and the triggers recording a table's changes into it. */
export interface LogDialect {
    /** Create the log, its horizon and its commit ordering once per database. */
    create(): readonly string[];
    /** Generate the triggers recording one table's committed changes. */
    install(description: ChangeDescription): string[];
    /** Remove the triggers recording one table's changes. */
    remove(description: ChangeDescription): string[];
}
