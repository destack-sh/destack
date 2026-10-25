import { TABLE, type Table } from "../table/table.ts";
import type { Column } from "../table/column.ts";
import type { ChangeDescription } from "./description.ts";
import { DatabaseError } from "../error/error.ts";
import { literal, quote } from "../dialect/quote.ts";
import { relation } from "../table/namespace.ts";

// NOTE #Performance: PgQ positions over a ticker's snapshots would remove the log's commit lock
/** The advisory lock class serialising a PostgreSQL log's commits, keyed by the log's schema. */
export const COMMIT_LOCK = 471_026_381;

/** The SQL name of the database's log. */
export const LOG = "__destack_log";

/** The SQL name of the highest compacted change sequence. */
export const LOG_HORIZON = "__destack_log_horizon";

/** The SQL name of the consumers' slots: each position, and when it stops retaining changes. */
export const LOG_SLOT = "__destack_log_slot";

/** The SQL name of the log's identity: its identifier, its epoch and the database's scope. */
export const LOG_EPOCH = "__destack_log_epoch";

/** The SQL name of the replica write marker, set while a replica's transaction writes. */
export const LOG_REPLICA = "__destack_log_replica";

/** The transaction-local setting with the time the connection's clock stamped on it. */
export const CHANGED_AT_SETTING = "destack.changed_at";

/** The transaction-local setting with the token a commit notification sends back to its writer. */
export const COMMIT_TOKEN_SETTING = "destack.commit_token";

/** The SQL name of the open SQLite transaction's identity. */
export const LOG_TRANSACTION = "__destack_log_transaction";

/** The log's tables, which have no change triggers. */
export const LOG_TABLES: readonly string[] = [
    LOG,
    LOG_HORIZON,
    LOG_SLOT,
    LOG_EPOCH,
    LOG_REPLICA,
    LOG_TRANSACTION,
];

/** The channel name of logged commits. */
export const LOG_TOPIC = "log";

/** The prefix of the PostgreSQL notification channels of a database's connections. */
export const CHANNEL_PREFIX = "destack_";

/** The prefix of a PostgreSQL schema's channel of logged commits, ended by the schema's hash. */
export const LOG_CHANNEL = `${CHANNEL_PREFIX}${LOG_TOPIC}`;

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
        throw DatabaseError.of(
            "MIGRATION_INVALID",
            `logged table has an optional scope column: ${definition.name}`,
        );
    }

    // record every column but sensitive ones under a key without binary columns
    const columns = Object.values(definition.columns).map((column) => column.definition);
    const logged = table[TABLE].logged;
    const recorded = Object.values(logged).map((column) => column.definition);
    const unlogged = definition.key.find(
        (property) =>
            !Object.hasOwn(logged, property) ||
            definition.column(property).definition.kind === "binary",
    );
    if (unlogged !== undefined) {
        throw DatabaseError.of(
            "MIGRATION_INVALID",
            `logged key column is binary or sensitive: ${definition.name}.${unlogged}`,
        );
    }

    return {
        table: definition.sqlName,
        retention: definition.retention,
        ...(definition.isAppendOnly ? { appendOnly: true as const } : {}),
        key: loggedKey(table).map((column) => column.definition.name),
        columns: recorded.map((column) => column.name),
        exact: recorded
            .filter((column) => column.kind === "bigint" || column.kind === "numeric")
            .map((column) => column.name),
        binary: recorded.filter((column) => column.kind === "binary").map((column) => column.name),
        compared: columns.map((column) => column.name),
        ...(scope === undefined ? {} : { scope: scope.definition.name }),
    };
}

/** Read a logged table's primary key columns in key order. */
export function loggedKey(table: Table): readonly Column[] {
    // require a key
    const columns = table[TABLE].key.map((property) => table[TABLE].column(property));
    if (columns.length === 0) {
        throw DatabaseError.of(
            "MIGRATION_INVALID",
            `logged table ${table[TABLE].sqlName} has no primary key: declare its changes as "none"`,
        );
    }

    return columns;
}

/** Create the log's slot, replica marker and epoch tables once. */
export function createEpoch(namespace?: string): readonly string[] {
    const name = (table: string) => quote(relation(table, namespace));

    return [
        `CREATE TABLE IF NOT EXISTS ${name(LOG_SLOT)} (
            name TEXT PRIMARY KEY,
            sequence BIGINT NOT NULL,
            expires_at BIGINT,
            claim BIGINT,
            claimed_at BIGINT,
            CHECK ((claim IS NULL) = (claimed_at IS NULL))
        )`,
        `CREATE TABLE IF NOT EXISTS ${name(LOG_REPLICA)} (
            slot INTEGER PRIMARY KEY CHECK (slot = 1)
        )`,
        `CREATE TABLE IF NOT EXISTS ${name(LOG_EPOCH)} (
            slot INTEGER PRIMARY KEY CHECK (slot = 1),
            log TEXT NOT NULL,
            epoch TEXT NOT NULL,
            scope TEXT
        )`,
    ];
}

/** Start the log once at a first epoch, its identifier, with the row scope of unscoped tables. */
export function insertEpoch(epoch: string, namespace?: string, scope?: string): string {
    const table = quote(relation(LOG_EPOCH, namespace));
    const scoped = scope === undefined ? "NULL" : literal(scope);

    return `INSERT INTO ${table} (slot, log, epoch, scope)
        VALUES (1, ${literal(epoch)}, ${literal(epoch)}, ${scoped})
        ON CONFLICT (slot) DO NOTHING`;
}

/** The log of one dialect: its tables and change triggers, within the database's namespace. */
export interface LogDialect {
    /** Create the log's tables and functions once within the database's namespace. */
    readonly create: (namespace?: string) => readonly string[];
    /** Generate the triggers on a table's relation recording its changes under its SQL name. */
    readonly install: (
        table: string,
        description: ChangeDescription,
        namespace?: string,
    ) => string[];
    /** Remove the triggers on a table's relation recording its changes. */
    readonly remove: (table: string) => string[];
}
