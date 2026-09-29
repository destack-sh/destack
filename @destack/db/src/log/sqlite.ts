import type { ChangeDescription } from "../inspect/log.ts";
import { literal, quote } from "../dialect/quote.ts";
import { createEpoch, LOG, LOG_HORIZON, LOG_TRANSACTION, type LogDialect } from "./schema.ts";

/** The most arguments of one SQLite function call, SQLITE_MAX_FUNCTION_ARG since SQLite 3.48. */
const FUNCTION_ARGUMENT_LIMIT = 1000;

/** The most key and value pairs of one JSON function call. */
const JSON_PAIR_LIMIT = Math.floor((FUNCTION_ARGUMENT_LIMIT - 1) / 2);

/** The SQLite log: its tables and a table's change triggers. */
export const sqliteLog: LogDialect = {
    create: () => createSQLiteLog(),
    install: (description) => sqliteLogTriggers(description),
    remove: (description) =>
        ["insert", "update", "move", "delete"].map(
            (suffix) => `DROP TRIGGER IF EXISTS ${quote(`${description.table}__change_${suffix}`)}`,
        ),
};

/** Create the SQLite log, its horizon and the transaction identity. */
function createSQLiteLog(): readonly string[] {
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
        `CREATE TABLE IF NOT EXISTS ${quote(LOG_TRANSACTION)} (
            slot INTEGER PRIMARY KEY CHECK (slot = 1),
            id TEXT NOT NULL
        )`,
        ...createEpoch(),
    ];
}

/** Generate the SQLite triggers recording one table's changes. */
function sqliteLogTriggers(description: ChangeDescription): string[] {
    // encode keys and columns as JSON, exact numbers as text
    const table = quote(description.table);
    const value = (row: "NEW" | "OLD", name: string) =>
        description.exact.includes(name)
            ? `CAST(${row}.${quote(name)} AS TEXT)`
            : `${row}.${quote(name)}`;
    const key = (row: "NEW" | "OLD") =>
        `json_array(${description.key.map((name) => value(row, name)).join(", ")})`;
    const row = (source: "NEW" | "OLD") => {
        const recorded = description.columns.map(
            (name) => `${literal(name)}, ${value(source, name)}`,
        );

        return recorded.length
            ? chunks(recorded, JSON_PAIR_LIMIT)
                  .map((chunk) => `json_object(${chunk.join(", ")})`)
                  .reduce((merged, next) => `json_patch(${merged}, ${next})`)
            : "json_object()";
    };
    const log = quote(LOG);
    const transaction = `(SELECT id FROM ${quote(LOG_TRANSACTION)} WHERE slot = 1)`;
    const now = "CAST(unixepoch('subsec') * 1000 AS INTEGER)";
    const scope = (source: "NEW" | "OLD") => `CAST(${source}.${quote(description.scope)} AS TEXT)`;

    // record each changed column's old value
    const previous = `json_remove(${chunks(
        description.columns.map(
            (name) =>
                `CASE WHEN NEW.${quote(name)} IS NOT OLD.${quote(name)} THEN ${literal(`$."${name.replaceAll('"', '\\"')}"`)} ELSE '$.__unchanged' END, ${value("OLD", name)}`,
        ),
        JSON_PAIR_LIMIT,
    ).reduce(
        (merged, chunk) => `json_insert(${merged}, ${chunk.join(", ")})`,
        "json_object()",
    )}, '$.__unchanged')`;
    const entry = (operation: string, source: "NEW" | "OLD", condition = "1 = 1", prior = "NULL") =>
        `INSERT INTO ${log} ("transaction", "table", key, operation, "row", previous, scope, retention, changed_at)
            SELECT
                ${transaction},
                ${literal(description.table)},
                ${key(source)},
                ${operation},
                ${row(source)},
                ${prior},
                ${scope(source)},
                ${literal(description.retention)},
                ${now}
            WHERE ${condition};`;
    const changed = description.compared
        .map((name) => `NEW.${quote(name)} IS NOT OLD.${quote(name)}`)
        .join(" OR ");
    const moved = [...description.key, description.scope]
        .map((name) => `NEW.${quote(name)} IS NOT OLD.${quote(name)}`)
        .join(" OR ");
    const prefix = `${description.table}__change`;

    // record a key or scope change as a deletion and an insertion
    return [
        `CREATE TRIGGER ${quote(`${prefix}_insert`)} AFTER INSERT ON ${table} BEGIN
            ${entry("'insert'", "NEW")}
        END`,
        `CREATE TRIGGER ${quote(`${prefix}_update`)} AFTER UPDATE ON ${table} WHEN (${changed}) AND NOT (${moved}) BEGIN
            ${entry("'update'", "NEW", "1 = 1", previous)}
        END`,
        `CREATE TRIGGER ${quote(`${prefix}_move`)} AFTER UPDATE ON ${table} WHEN ${moved} BEGIN
            ${entry("'delete'", "OLD")}
            ${entry("'insert'", "NEW")}
        END`,
        `CREATE TRIGGER ${quote(`${prefix}_delete`)} AFTER DELETE ON ${table} BEGIN
            ${entry("'delete'", "OLD")}
        END`,
    ];
}

/** Split values into groups within the argument limit. */
function chunks<Value>(values: readonly Value[], size: number): Value[][] {
    const groups: Value[][] = [];
    for (let start = 0; start < values.length; start += size) {
        groups.push(values.slice(start, start + size));
    }

    return groups;
}
