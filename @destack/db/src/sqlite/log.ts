import type { ChangeDescription } from "../inspect/log.ts";
import { literal, quote } from "../dialect/quote.ts";
import {
    createEpoch,
    LOG,
    LOG_EPOCH,
    LOG_HORIZON,
    LOG_TRANSACTION,
    type LogDialect,
} from "../log/schema.ts";

/** The most arguments of one SQLite function call, SQLITE_MAX_FUNCTION_ARG since SQLite 3.48. */
const FUNCTION_ARGUMENT_LIMIT = 1000;

/** The most key and value pairs of one JSON function call. */
const JSON_PAIR_LIMIT = Math.floor((FUNCTION_ARGUMENT_LIMIT - 1) / 2);

/** The SQLite log: its tables and a table's change triggers. */
export const sqliteLog: LogDialect = {
    create: (epoch, scope) => createSQLiteLog(epoch, scope),
    install: (description) => sqliteLogTriggers(description),
    remove: (description) =>
        ["insert", "update", "move", "delete"].map(
            (suffix) => `DROP TRIGGER IF EXISTS ${quote(`${description.table}__change_${suffix}`)}`,
        ),
};

/** Create the SQLite log, its horizon and the transaction identity. */
function createSQLiteLog(epoch: string, scope?: string): readonly string[] {
    return [
        `CREATE TABLE IF NOT EXISTS ${quote(LOG)} (
            sequence INTEGER PRIMARY KEY AUTOINCREMENT,
            "transaction" TEXT,
            "table" TEXT NOT NULL,
            key TEXT NOT NULL,
            operation TEXT NOT NULL,
            "row" TEXT NOT NULL,
            previous TEXT,
            scope TEXT NOT NULL,
            retention TEXT NOT NULL,
            changed_at INTEGER NOT NULL
        )`,
        `CREATE INDEX IF NOT EXISTS ${quote(`${LOG}_compaction`)} ON ${quote(LOG)}(retention, changed_at)`,
        `CREATE INDEX IF NOT EXISTS ${quote(`${LOG}_scope`)} ON ${quote(LOG)}(scope, sequence)`,
        `CREATE INDEX IF NOT EXISTS ${quote(`${LOG}_transaction_sequence`)} ON ${quote(LOG)}("transaction", sequence)`,
        `CREATE TABLE IF NOT EXISTS ${quote(LOG_HORIZON)} (
            slot INTEGER PRIMARY KEY CHECK (slot = 1),
            sequence INTEGER NOT NULL
        )`,
        `INSERT INTO ${quote(LOG_HORIZON)} (slot, sequence) VALUES (1, 0) ON CONFLICT (slot) DO NOTHING`,
        `CREATE TABLE IF NOT EXISTS ${quote(LOG_TRANSACTION)} (
            slot INTEGER PRIMARY KEY CHECK (slot = 1),
            id TEXT NOT NULL
        )`,
        ...createEpoch(epoch, scope),
    ];
}

/** Generate the SQLite triggers recording one table's changes. */
function sqliteLogTriggers(description: ChangeDescription): string[] {
    // abort a change of a table taking the database's scope while the database has none
    const table = quote(description.table);
    const scoped =
        description.scope === undefined
            ? `SELECT RAISE(ABORT, ${literal(`the database has no scope for the rows of ${description.table}`)}) WHERE ${databaseScope()} IS NULL;`
            : "";

    // tell updates from key or scope changes
    const changed = anyChanged(description.compared);
    const moved = anyChanged([
        ...description.key,
        ...(description.scope === undefined ? [] : [description.scope]),
    ]);
    const prefix = `${description.table}__change`;

    // record a key or scope change as a deletion and an insertion
    return [
        `CREATE TRIGGER ${quote(`${prefix}_insert`)} AFTER INSERT ON ${table} BEGIN
            ${scoped}
            ${logEntry(description, "'insert'", "NEW")}
        END`,
        `CREATE TRIGGER ${quote(`${prefix}_update`)} AFTER UPDATE ON ${table} WHEN (${changed}) AND NOT (${moved}) BEGIN
            ${scoped}
            ${logEntry(description, "'update'", "NEW", previousValues(description))}
        END`,
        `CREATE TRIGGER ${quote(`${prefix}_move`)} AFTER UPDATE ON ${table} WHEN ${moved} BEGIN
            ${scoped}
            ${logEntry(description, "'delete'", "OLD")}
            ${logEntry(description, "'insert'", "NEW")}
        END`,
        `CREATE TRIGGER ${quote(`${prefix}_delete`)} AFTER DELETE ON ${table} BEGIN
            ${scoped}
            ${logEntry(description, "'delete'", "OLD")}
        END`,
    ];
}

/** Insert one log entry of a row image. */
function logEntry(
    description: ChangeDescription,
    operation: string,
    source: "NEW" | "OLD",
    prior = "NULL",
): string {
    const transaction = `(SELECT id FROM ${quote(LOG_TRANSACTION)} WHERE slot = 1)`;
    const now = "CAST(unixepoch('subsec') * 1000 AS INTEGER)";

    return `INSERT INTO ${quote(LOG)} ("transaction", "table", key, operation, "row", previous, scope, retention, changed_at)
            SELECT
                ${transaction},
                ${literal(description.table)},
                ${recordedKey(description, source)},
                ${operation},
                ${recordedRow(description, source)},
                ${prior},
                ${rowScope(description, source)},
                ${literal(description.retention)},
                ${now};`;
}

/** Read a recorded column of a row image as JSON takes it. */
function recordedValue(
    description: ChangeDescription,
    source: "NEW" | "OLD",
    name: string,
): string {
    const column = `${source}.${quote(name)}`;
    // read an exact number as text
    if (description.exact.includes(name)) {
        return `CAST(${column} AS TEXT)`;
    }
    // read bytes as hexadecimal text
    else if (description.binary.includes(name)) {
        return `CASE WHEN ${column} IS NULL THEN NULL ELSE lower(hex(${column})) END`;
    }
    // read any other value as it is
    else {
        return column;
    }
}

/** Encode a row image's key as a JSON array. */
function recordedKey(description: ChangeDescription, source: "NEW" | "OLD"): string {
    const values = description.key.map((name) => recordedValue(description, source, name));

    return `json_array(${values.join(", ")})`;
}

/** Encode a row image's recorded columns as a JSON object. */
function recordedRow(description: ChangeDescription, source: "NEW" | "OLD"): string {
    const recorded = description.columns.map(
        (name) => `${literal(name)}, ${recordedValue(description, source, name)}`,
    );

    return recorded.length
        ? chunks(recorded, JSON_PAIR_LIMIT)
              .map((chunk) => `json_object(${chunk.join(", ")})`)
              .reduce((merged, next) => `json_patch(${merged}, ${next})`)
        : "json_object()";
}

/** Encode each changed column's old value as a JSON object. */
function previousValues(description: ChangeDescription): string {
    const pairs = description.columns.map(
        (name) =>
            `CASE WHEN NEW.${quote(name)} IS NOT OLD.${quote(name)} THEN ${literal(`$."${name.replaceAll('"', '\\"')}"`)} ELSE '$.__unchanged' END, ${recordedValue(description, "OLD", name)}`,
    );
    const inserted = chunks(pairs, JSON_PAIR_LIMIT).reduce(
        (merged, chunk) => `json_insert(${merged}, ${chunk.join(", ")})`,
        "json_object()",
    );

    return `json_remove(${inserted}, '$.__unchanged')`;
}

/** Read a row image's scope. */
function rowScope(description: ChangeDescription, source: "NEW" | "OLD"): string {
    return description.scope === undefined
        ? databaseScope()
        : `CAST(${source}.${quote(description.scope)} AS TEXT)`;
}

/** Read the database's scope. */
function databaseScope(): string {
    return `(SELECT scope FROM ${quote(LOG_EPOCH)} WHERE slot = 1)`;
}

/** Match a row whose columns changed in any of some columns. */
function anyChanged(names: readonly string[]): string {
    return names.map((name) => `NEW.${quote(name)} IS NOT OLD.${quote(name)}`).join(" OR ");
}

/** Split values into groups within the argument limit. */
function chunks<Value>(values: readonly Value[], size: number): Value[][] {
    const groups: Value[][] = [];
    for (let start = 0; start < values.length; start += size) {
        groups.push(values.slice(start, start + size));
    }

    return groups;
}
