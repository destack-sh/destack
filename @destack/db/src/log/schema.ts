import { TABLE, type Table } from "../table/table.ts";
import type { Column } from "../table/column.ts";
import type { ChangeDescription } from "../inspect/log.ts";
import { DatabaseError } from "../error/error.ts";
import { literal, quote } from "../dialect/quote.ts";

/** The SQL name of the database's log. */
export const LOG = "__destack_log";

/** The SQL name of the highest compacted change sequence. */
export const LOG_HORIZON = "__destack_log_horizon";

/** The SQL name of the consumers' slots: each one's position, and when it stops keeping changes. */
export const LOG_SLOT = "__destack_log_slot";

/** The SQL name of the log's epoch. */
export const LOG_EPOCH = "__destack_log_epoch";

/** The SQL name of the marker of a transaction writing as a replica. */
export const LOG_REPLICA = "__destack_log_replica";

/** The SQL name of the open SQLite transaction's identity. */
export const LOG_TRANSACTION = "__destack_log_transaction";

/** The channel name of logged commits. */
export const LOG_TOPIC = "log";

/** The prefix of the PostgreSQL notification channels of a database's connections. */
export const CHANNEL_PREFIX = "destack_";

/** The PostgreSQL notification channel of logged commits. */
export const LOG_CHANNEL = `${CHANNEL_PREFIX}${LOG_TOPIC}`;

/** The SQL names of every table the log keeps. */
export const LOG_TABLES = [
    LOG,
    LOG_HORIZON,
    LOG_SLOT,
    LOG_EPOCH,
    LOG_REPLICA,
    LOG_TRANSACTION,
] as const;

/** Describe the columns a table's change triggers record, absent for unlogged tables. */
export function describeLog(table: Table): ChangeDescription | undefined {
    // skip unlogged tables
    const definition = table[TABLE];
    if (definition.retention === "none") {
        return undefined;
    }

    // file each change under the row's scope, or the database's for a table without one
    const scope = definition.columns["scope"];
    if (scope?.definition.nullable === true) {
        throw new DatabaseError(
            "INVALID_MIGRATION",
            `logged table has an optional scope column: ${definition.name}`,
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
        key: loggedKey(table).map((column) => column.definition.name),
        columns: recorded.map((column) => column.name),
        exact: recorded
            .filter((column) => column.kind === "bigint" || column.kind === "numeric")
            .map((column) => column.name),
        compared: columns.map((column) => column.name),
        ...(scope === undefined ? {} : { scope: scope.definition.name }),
    };
}

/** Read a logged table's primary key columns in key order. */
export function loggedKey(table: Table): readonly Column[] {
    // require a key
    const columns = table[TABLE].key.map((property) => table[TABLE].column(property));
    if (columns.length === 0) {
        throw new DatabaseError(
            "INVALID_MIGRATION",
            `logged table has no primary key: ${table[TABLE].sqlName}; declare its changes as "none"`,
        );
    }

    return columns;
}

/** Create the log's first epoch once, with the scope of the rows in tables without a scope column, and keep existing state. */
export function createEpoch(epoch: string, scope?: string): readonly string[] {
    return [
        `CREATE TABLE IF NOT EXISTS ${quote(LOG_SLOT)} (
            name TEXT PRIMARY KEY,
            sequence BIGINT NOT NULL,
            expires_at BIGINT NOT NULL
        )`,
        `CREATE TABLE IF NOT EXISTS ${quote(LOG_REPLICA)} (slot INTEGER PRIMARY KEY CHECK (slot = 1))`,
        `CREATE TABLE IF NOT EXISTS ${quote(LOG_EPOCH)} (
            slot INTEGER PRIMARY KEY CHECK (slot = 1),
            epoch TEXT NOT NULL,
            scope TEXT
        )`,
        `INSERT INTO ${quote(LOG_EPOCH)} (slot, epoch, scope)
            VALUES (1, ${literal(epoch)}, ${scope === undefined ? "NULL" : literal(scope)})
            ON CONFLICT (slot) DO NOTHING`,
    ];
}

/** The log of one dialect: its tables and change triggers. */
export interface LogDialect {
    /** Create the log once per database at a first epoch, with the scope of the rows in tables without a scope column. */
    create(epoch: string, scope?: string): readonly string[];
    /** Generate the triggers recording one table's changes. */
    install(description: ChangeDescription): string[];
    /** Remove the triggers recording one table's changes. */
    remove(description: ChangeDescription): string[];
}
