import type { Triggers } from "../migration/trigger.ts";
import { TABLE, type Table } from "../table/table.ts";
import type { Column } from "../table/column.ts";
import type { PrimaryKey } from "../table/constraint.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { ChangeDescription } from "../inspect/log.ts";
import { assertNever, DatabaseError } from "../error/error.ts";
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

/** The advisory lock key serialising PostgreSQL commit stamping, an arbitrary key no other lock uses. */
const COMMIT_LOCK = 4_710_263_811;

/** The most arguments one SQLite function call takes, SQLITE_MAX_FUNCTION_ARG since SQLite 3.48. */
const FUNCTION_ARGUMENT_LIMIT = 1000;

/** The most key and value pairs one JSON function call takes after its target argument. */
const JSON_PAIR_LIMIT = Math.floor((FUNCTION_ARGUMENT_LIMIT - 1) / 2);

/** The PostgreSQL notification channel announcing commits that changed the log. */
export const LOG_CHANNEL = "destack_log";

/** Describe the columns a table's change triggers record, absent for unlogged tables. */
export function describeLog(table: Table): ChangeDescription | undefined {
    // skip unlogged tables
    const definition = table[TABLE];
    if (definition.tier === "none") {
        return undefined;
    }

    // require the route to name a column
    const route = definition.route;
    if (route !== undefined && !Object.hasOwn(definition.columns, route)) {
        throw new DatabaseError(
            "INVALID_MIGRATION",
            `unknown route column: ${definition.name}.${route}`,
        );
    }

    // record every column except binary and sensitive ones
    const columns = Object.values(definition.columns).map((column) => column.definition);
    const recorded = Object.values(loggedColumns(table)).map((column) => column.definition);

    return {
        table: definition.sqlName,
        tier: definition.tier,
        key: primaryKey(table).map((column) => column.definition.name),
        columns: recorded.map((column) => column.name),
        exact: recorded
            .filter((column) => column.kind === "bigint" || column.kind === "numeric")
            .map((column) => column.name),
        compared: columns.map((column) => column.name),
        ...(route === undefined ? {} : { route: definition.columns[route]!.definition.name }),
    };
}

/** Select the columns a table's log records, by property: every column except binary and sensitive ones. */
export function loggedColumns(table: Table): Record<string, Column> {
    return Object.fromEntries(
        Object.entries(table[TABLE].columns).filter(
            ([, column]) =>
                column.definition.kind !== "binary" &&
                column.definition.classification !== "sensitive",
        ),
    );
}

/** Read a logged table's primary key columns in key order. */
export function primaryKey(table: Table): readonly Column[] {
    // prefer a compound key constraint over column-level keys
    const declared = table
        .constraints("sqlite")
        .find((constraint): constraint is PrimaryKey => constraint.kind === "primaryKey");
    const columns =
        declared?.columns ??
        Object.values(table[TABLE].columns).filter((column) => column.definition.primaryKey);
    if (columns.length === 0) {
        throw new DatabaseError(
            "INVALID_MIGRATION",
            `logged table has no primary key: ${table[TABLE].sqlName}; declare its changes as "none"`,
        );
    }

    return columns;
}

/** Create the log, its horizon and commit ordering once per database. */
export function createLog(dialect: Dialect): readonly string[] {
    // create the SQLite log tables
    if (dialect === "sqlite") {
        return createSQLiteLog();
    }
    // create the PostgreSQL log tables and commit stamping
    else if (dialect === "postgresql") {
        return createPostgresLog();
    }
    // reject other dialects
    else {
        return assertNever(dialect);
    }
}

/** Create the SQLite log, its horizon and the open transaction's identity. */
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
            route TEXT,
            tier TEXT NOT NULL,
            changed_at INTEGER NOT NULL
        )`,
        `CREATE INDEX IF NOT EXISTS ${quote(`${LOG}_compaction`)} ON ${quote(LOG)}(tier, changed_at)`,
        `CREATE INDEX IF NOT EXISTS ${quote(`${LOG}_route`)} ON ${quote(LOG)}(route, sequence) WHERE route IS NOT NULL`,
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

/** Create the log's epoch once, keeping the epoch an earlier creation minted, and the copying marker. */
function createEpoch(): readonly string[] {
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

/** Create the PostgreSQL log, its horizon and the functions stamping and recording changes. */
function createPostgresLog(): readonly string[] {
    const log = quote(LOG);

    // stamp sequences at commit under one lock, so sequence order is commit order
    return [
        `CREATE TABLE IF NOT EXISTS ${log} (
            id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
            sequence BIGINT UNIQUE,
            "transaction" TEXT NOT NULL,
            "table" TEXT NOT NULL,
            key JSONB NOT NULL,
            operation TEXT NOT NULL,
            "row" JSONB NOT NULL,
            previous JSONB,
            route TEXT,
            tier TEXT NOT NULL,
            changed_at BIGINT NOT NULL
        )`,
        `CREATE INDEX IF NOT EXISTS ${quote(`${LOG}_compaction`)} ON ${log}(tier, changed_at)`,
        `CREATE INDEX IF NOT EXISTS ${quote(`${LOG}_route`)} ON ${log}(route, sequence) WHERE route IS NOT NULL`,
        `CREATE SEQUENCE IF NOT EXISTS ${quote(`${LOG}_sequence`)}`,
        `CREATE TABLE IF NOT EXISTS ${quote(LOG_HORIZON)} (
            slot INTEGER PRIMARY KEY CHECK (slot = 1),
            sequence BIGINT NOT NULL
        )`,
        ...createEpoch(),
        `CREATE OR REPLACE FUNCTION ${quote(`${LOG}_stamp`)}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
            PERFORM pg_advisory_xact_lock(${COMMIT_LOCK});
            UPDATE ${log} SET sequence = nextval('${LOG}_sequence') WHERE id = NEW.id;
            PERFORM pg_notify('${LOG_CHANNEL}', '');
            RETURN NULL;
        END $$`,
        `DO $$ BEGIN
            IF NOT EXISTS (SELECT FROM pg_trigger WHERE tgname = '${LOG}_stamp' AND tgrelid = '${log}'::regclass) THEN
                CREATE CONSTRAINT TRIGGER ${quote(`${LOG}_stamp`)} AFTER INSERT ON ${log}
                    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ${quote(`${LOG}_stamp`)}();
            END IF;
        END $$`,
        `CREATE OR REPLACE FUNCTION ${quote(`${LOG}_record`)}() RETURNS trigger LANGUAGE plpgsql AS $$
        DECLARE
            tier TEXT := TG_ARGV[0];
            key_columns TEXT[] := TG_ARGV[1]::TEXT[];
            recorded TEXT[] := TG_ARGV[2]::TEXT[];
            exact TEXT[] := TG_ARGV[3]::TEXT[];
            route_column TEXT := NULLIF(TG_ARGV[4], '');
            old_route TEXT;
            new_route TEXT;
            previous JSONB;
            now_ms BIGINT := floor(extract(epoch FROM clock_timestamp()) * 1000);
            transaction_id TEXT := pg_current_xact_id()::TEXT;
            old_row JSONB;
            new_row JSONB;
            old_key JSONB;
            new_key JSONB;
            old_recorded JSONB;
            new_recorded JSONB;
            name TEXT;
        BEGIN
            IF TG_OP <> 'INSERT' THEN old_row := to_jsonb(OLD); END IF;
            IF TG_OP <> 'DELETE' THEN new_row := to_jsonb(NEW); END IF;
            IF TG_OP = 'UPDATE' AND old_row = new_row THEN RETURN NULL; END IF;
            IF old_row IS NOT NULL THEN
                SELECT jsonb_agg(old_row -> column_name ORDER BY position) INTO old_key
                FROM unnest(key_columns) WITH ORDINALITY AS entry(column_name, position);
                SELECT jsonb_object_agg(column_name, old_row -> column_name) INTO old_recorded
                FROM unnest(recorded) AS entry(column_name);
                FOREACH name IN ARRAY exact LOOP
                    IF old_recorded -> name <> 'null'::jsonb THEN
                        old_recorded := jsonb_set(old_recorded, ARRAY[name], to_jsonb(old_recorded ->> name));
                    END IF;
                END LOOP;
            END IF;
            IF new_row IS NOT NULL THEN
                SELECT jsonb_agg(new_row -> column_name ORDER BY position) INTO new_key
                FROM unnest(key_columns) WITH ORDINALITY AS entry(column_name, position);
                SELECT jsonb_object_agg(column_name, new_row -> column_name) INTO new_recorded
                FROM unnest(recorded) AS entry(column_name);
                FOREACH name IN ARRAY exact LOOP
                    IF new_recorded -> name <> 'null'::jsonb THEN
                        new_recorded := jsonb_set(new_recorded, ARRAY[name], to_jsonb(new_recorded ->> name));
                    END IF;
                END LOOP;
            END IF;
            IF route_column IS NOT NULL THEN
                old_route := old_row ->> route_column;
                new_route := new_row ->> route_column;
            END IF;
            IF TG_OP = 'UPDATE' AND old_key = new_key AND old_route IS NOT DISTINCT FROM new_route THEN
                SELECT jsonb_object_agg(entry.key, entry.value) INTO previous
                FROM jsonb_each(old_recorded) AS entry
                WHERE entry.value IS DISTINCT FROM new_recorded -> entry.key;
            END IF;
            IF TG_OP = 'DELETE' OR (TG_OP = 'UPDATE' AND (old_key <> new_key OR old_route IS DISTINCT FROM new_route)) THEN
                INSERT INTO ${log}("transaction", "table", key, operation, "row", route, tier, changed_at)
                VALUES (transaction_id, TG_TABLE_NAME, old_key, 'delete', old_recorded, old_route, tier, now_ms);
            END IF;
            IF TG_OP <> 'DELETE' THEN
                INSERT INTO ${log}("transaction", "table", key, operation, "row", previous, route, tier, changed_at)
                VALUES (transaction_id, TG_TABLE_NAME, new_key,
                    CASE WHEN TG_OP = 'INSERT' OR old_key <> new_key OR old_route IS DISTINCT FROM new_route THEN 'insert' ELSE 'update' END,
                    new_recorded, previous, new_route, tier, now_ms);
            END IF;
            RETURN NULL;
        END $$`,
    ];
}

/** Generate the triggers recording one table's committed changes. */
function install(description: ChangeDescription, dialect: Dialect): string[] {
    // call the shared PostgreSQL recording function
    if (dialect === "postgresql") {
        return postgresLogTriggers(description);
    }
    // record each SQLite change in its own triggers
    else if (dialect === "sqlite") {
        return sqliteLogTriggers(description);
    }
    // reject other dialects
    else {
        return assertNever(dialect);
    }
}

/** Pass the key, recorded and exact columns and the route to the shared PostgreSQL function. */
function postgresLogTriggers(description: ChangeDescription): string[] {
    // write the column lists as PostgreSQL array literals
    const table = quote(description.table);
    const routeColumn = description.route === undefined ? "" : description.route;
    const array = (names: readonly string[]) =>
        `{${names.map((name) => `"${name.replaceAll('"', '\\"')}"`).join(",")}}`;

    // pass the tier, key, recorded and exact columns and the route column
    const parameters = [
        literal(description.tier),
        literal(array(description.key)),
        literal(array(description.columns)),
        literal(array(description.exact)),
        literal(routeColumn),
    ];

    return [
        `CREATE TRIGGER ${quote("destack_change")}
            AFTER INSERT OR UPDATE OR DELETE ON ${table}
            FOR EACH ROW
            EXECUTE FUNCTION ${quote(`${LOG}_record`)}(${parameters.join(", ")})`,
    ];
}

/** Generate the SQLite triggers recording one table's changes as JSON. */
function sqliteLogTriggers(description: ChangeDescription): string[] {
    // encode keys and recorded columns as JSON, keeping exact numbers as text
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
    const route = (source: "NEW" | "OLD") =>
        description.route === undefined
            ? "NULL"
            : `CAST(${source}.${quote(description.route)} AS TEXT)`;

    // insert each changed column's old value, null included, and drop the unchanged ones
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
        `INSERT INTO ${log} ("transaction", "table", key, operation, "row", previous, route, tier, changed_at)
            SELECT
                ${transaction},
                ${literal(description.table)},
                ${key(source)},
                ${operation},
                ${row(source)},
                ${prior},
                ${route(source)},
                ${literal(description.tier)},
                ${now}
            WHERE ${condition};`;
    const changed = description.compared
        .map((name) => `NEW.${quote(name)} IS NOT OLD.${quote(name)}`)
        .join(" OR ");
    const moved = [
        ...description.key,
        ...(description.route === undefined ? [] : [description.route]),
    ]
        .map((name) => `NEW.${quote(name)} IS NOT OLD.${quote(name)}`)
        .join(" OR ");
    const prefix = `${description.table}__change`;

    // record a key or route change as the old row's deletion and the new row's insertion
    return [
        `CREATE TRIGGER ${quote(`${prefix}_insert`)} AFTER INSERT ON ${table} BEGIN
            ${entry("'insert'", "NEW")}
        END`,
        `CREATE TRIGGER ${quote(`${prefix}_update`)} AFTER UPDATE ON ${table} WHEN ${changed} BEGIN
            ${entry("'delete'", "OLD", `(${moved})`)}
            ${entry(
                `CASE WHEN ${moved} THEN 'insert' ELSE 'update' END`,
                "NEW",
                "1 = 1",
                `CASE WHEN ${moved} THEN NULL ELSE ${previous} END`,
            )}
        END`,
        `CREATE TRIGGER ${quote(`${prefix}_delete`)} AFTER DELETE ON ${table} BEGIN
            ${entry("'delete'", "OLD")}
        END`,
    ];
}

/** Remove the triggers recording one table's changes. */
function remove(description: ChangeDescription, dialect: Dialect): string[] {
    // drop the PostgreSQL trigger
    if (dialect === "postgresql") {
        return [`DROP TRIGGER IF EXISTS ${quote("destack_change")} ON ${quote(description.table)}`];
    }
    // drop each SQLite trigger
    else if (dialect === "sqlite") {
        return ["insert", "update", "delete"].map(
            (suffix) => `DROP TRIGGER IF EXISTS ${quote(`${description.table}__change_${suffix}`)}`,
        );
    }
    // reject other dialects
    else {
        return assertNever(dialect);
    }
}

/** Split values into groups within a SQL function's argument limit. */
function chunks<Value>(values: readonly Value[], size: number): Value[][] {
    const groups: Value[][] = [];
    for (let start = 0; start < values.length; start += size) {
        groups.push(values.slice(start, start + size));
    }

    return groups;
}

/** The change triggers of a logged table. */
export const logTriggers: Triggers = {
    install: (state, dialect) => (state.log ? install(state.log, dialect) : []),
    remove: (state, dialect) => (state.log ? remove(state.log, dialect) : []),
};
