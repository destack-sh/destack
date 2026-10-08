import type { ChangeDescription } from "../log/description.ts";
import { literal, quote } from "../dialect/quote.ts";
import {
    createEpoch,
    LOG,
    LOG_CHANNEL,
    LOG_EPOCH,
    LOG_HORIZON,
    LOG_REPLICA,
    type LogDialect,
} from "../log/schema.ts";

/** The transaction-local setting naming the already stamped transaction. */
const STAMPED_SETTING = "destack.stamped";

/** The bytes of table names one commit notification carries: below PostgreSQL's payload limit of 8000, leaving room for the message around them. */
const NOTIFICATION_BYTES = 7_800;

// TODO #Architecture: replace the one lock per log with PgQ positions over a ticker's snapshots
/** The advisory lock class serialising PostgreSQL commit stamping, keyed by the log's schema. */
const COMMIT_LOCK = 471_026_381;

/** The PostgreSQL log: its tables, functions and triggers. */
export const postgresLog: LogDialect = {
    create: (epoch, scope) => createPostgresLog(epoch, scope),
    install: (table, description) => postgresLogTriggers(table, description),
    remove: (table) => [
        `DROP TRIGGER IF EXISTS ${quote("destack_change")} ON ${quote(table)}`,
        `DROP TRIGGER IF EXISTS ${quote("destack_append")} ON ${quote(table)}`,
    ],
};

/** Create the PostgreSQL log, its horizon and its functions. */
function createPostgresLog(epoch: string, scope?: string): readonly string[] {
    // create the log with its stamp and record functions
    return [...createLogTables(), ...createEpoch(epoch, scope), ...createStamp(), createRecord()];
}

/** Create the log table with its indexes and sequence and the horizon. */
function createLogTables(): string[] {
    const log = quote(LOG);

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
            scope TEXT NOT NULL,
            retention TEXT NOT NULL,
            changed_at BIGINT NOT NULL,
            origin TEXT
        )`,
        `CREATE INDEX IF NOT EXISTS ${quote(`${LOG}_compaction`)} ON ${log}(retention, changed_at)`,
        `CREATE INDEX IF NOT EXISTS ${quote(`${LOG}_scope`)} ON ${log}(scope, sequence)`,
        `CREATE INDEX IF NOT EXISTS ${quote(`${LOG}_transaction_sequence`)} ON ${log}("transaction", sequence)`,
        `CREATE INDEX IF NOT EXISTS ${quote(`${LOG}_unstamped`)} ON ${log}("transaction", id) WHERE sequence IS NULL`,
        `CREATE SEQUENCE IF NOT EXISTS ${quote(`${LOG}_sequence`)}`,
        `CREATE TABLE IF NOT EXISTS ${quote(LOG_HORIZON)} (
            slot INTEGER PRIMARY KEY CHECK (slot = 1),
            sequence BIGINT NOT NULL
        )`,
        `INSERT INTO ${quote(LOG_HORIZON)} (slot, sequence) VALUES (1, 0) ON CONFLICT (slot) DO NOTHING`,
    ];
}

/** Create the function and deferred trigger numbering a transaction's entries at commit in commit order, and announcing their tables. */
function createStamp(): string[] {
    const log = quote(LOG);

    return [
        `CREATE OR REPLACE FUNCTION ${quote(`${LOG}_stamp`)}() RETURNS trigger LANGUAGE plpgsql AS $$
        DECLARE
            unstamped BIGINT;
            base BIGINT;
            tables TEXT[];
            batch TEXT[] := '{}';
            bytes INTEGER := 0;
            name TEXT;
        BEGIN
            IF current_setting('${STAMPED_SETTING}', true) = NEW."transaction" THEN RETURN NULL; END IF;
            PERFORM pg_advisory_xact_lock(${COMMIT_LOCK}, hashtext(TG_TABLE_SCHEMA));
            SELECT count(*), array_agg(DISTINCT "table") INTO unstamped, tables
            FROM ${log} WHERE "transaction" = NEW."transaction" AND sequence IS NULL;
            base := nextval('${LOG}_sequence');
            PERFORM setval('${LOG}_sequence', base + unstamped - 1);
            UPDATE ${log} SET sequence = numbered.sequence
            FROM (
                SELECT id, base + row_number() OVER (ORDER BY id) - 1 AS sequence
                FROM ${log} WHERE "transaction" = NEW."transaction" AND sequence IS NULL
            ) AS numbered
            WHERE ${log}.id = numbered.id;
            FOREACH name IN ARRAY tables LOOP
                IF bytes + octet_length(name) + 3 > ${NOTIFICATION_BYTES} THEN
                    PERFORM pg_notify('${LOG_CHANNEL}', json_build_object('kind', 'commit', 'tables', batch)::TEXT);
                    batch := '{}';
                    bytes := 0;
                END IF;
                batch := batch || name;
                bytes := bytes + octet_length(name) + 3;
            END LOOP;
            PERFORM pg_notify('${LOG_CHANNEL}', json_build_object('kind', 'commit', 'tables', batch)::TEXT);
            PERFORM set_config('${STAMPED_SETTING}', NEW."transaction", true);
            RETURN NULL;
        END $$`,
        `DO $$ BEGIN
            IF NOT EXISTS (SELECT FROM pg_trigger WHERE tgname = '${LOG}_stamp' AND tgrelid = '${log}'::regclass) THEN
                CREATE CONSTRAINT TRIGGER ${quote(`${LOG}_stamp`)} AFTER INSERT ON ${log}
                    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ${quote(`${LOG}_stamp`)}();
            END IF;
        END $$`,
    ];
}

/** Create the function recording a row change. */
function createRecord(): string {
    const log = quote(LOG);

    return `CREATE OR REPLACE FUNCTION ${quote(`${LOG}_record`)}() RETURNS trigger LANGUAGE plpgsql SET bytea_output = 'hex' AS $$
        DECLARE
            retention TEXT := TG_ARGV[0];
            key_columns TEXT[] := TG_ARGV[1]::TEXT[];
            recorded TEXT[] := TG_ARGV[2]::TEXT[];
            exact TEXT[] := TG_ARGV[3]::TEXT[];
            binary_columns TEXT[] := TG_ARGV[4]::TEXT[];
            scope_column TEXT := TG_ARGV[5];
            database_scope TEXT;
            old_scope TEXT;
            new_scope TEXT;
            previous JSONB;
            now_ms BIGINT := floor(extract(epoch FROM clock_timestamp()) * 1000);
            transaction_id TEXT := pg_current_xact_id()::TEXT;
            origin TEXT;
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
            ${recordedImage("old")}
            ${recordedImage("new")}
            SELECT marker.origin INTO origin FROM ${quote(LOG_REPLICA)} AS marker WHERE marker.slot = 1;
            IF scope_column IS NULL THEN
                SELECT scope INTO database_scope FROM ${quote(LOG_EPOCH)} WHERE slot = 1;
                IF database_scope IS NULL THEN
                    RAISE EXCEPTION 'the database has no scope for the rows of %', TG_TABLE_NAME;
                END IF;
                IF old_row IS NOT NULL THEN old_scope := database_scope; END IF;
                IF new_row IS NOT NULL THEN new_scope := database_scope; END IF;
            ELSE
                old_scope := old_row ->> scope_column;
                new_scope := new_row ->> scope_column;
            END IF;
            IF TG_OP = 'UPDATE' AND old_key = new_key AND old_scope IS NOT DISTINCT FROM new_scope THEN
                SELECT jsonb_object_agg(entry.key, entry.value) INTO previous
                FROM jsonb_each(old_recorded) AS entry
                WHERE entry.value IS DISTINCT FROM new_recorded -> entry.key;
            END IF;
            IF TG_OP = 'DELETE' OR (TG_OP = 'UPDATE' AND (old_key <> new_key OR old_scope IS DISTINCT FROM new_scope)) THEN
                INSERT INTO ${log}("transaction", "table", key, operation, "row", scope, retention, changed_at, origin)
                VALUES (transaction_id, TG_TABLE_NAME, old_key, 'delete', old_recorded, old_scope, retention, now_ms, origin);
            END IF;
            IF TG_OP <> 'DELETE' THEN
                INSERT INTO ${log}("transaction", "table", key, operation, "row", previous, scope, retention, changed_at, origin)
                VALUES (transaction_id, TG_TABLE_NAME, new_key,
                    CASE WHEN TG_OP = 'INSERT' OR old_key <> new_key OR old_scope IS DISTINCT FROM new_scope THEN 'insert' ELSE 'update' END,
                    new_recorded, previous, new_scope, retention, now_ms, origin);
            END IF;
            RETURN NULL;
        END $$`;
}

/** Read one row image's key and recorded columns. */
function recordedImage(side: "old" | "new"): string {
    return `IF ${side}_row IS NOT NULL THEN
                SELECT jsonb_agg(${side}_row -> column_name ORDER BY position) INTO ${side}_key
                FROM unnest(key_columns) WITH ORDINALITY AS entry(column_name, position);
                SELECT jsonb_object_agg(column_name, ${side}_row -> column_name) INTO ${side}_recorded
                FROM unnest(recorded) AS entry(column_name);
                FOREACH name IN ARRAY exact LOOP
                    IF ${side}_recorded -> name <> 'null'::jsonb THEN
                        ${side}_recorded := jsonb_set(${side}_recorded, ARRAY[name], to_jsonb(${side}_recorded ->> name));
                    END IF;
                END LOOP;
                FOREACH name IN ARRAY binary_columns LOOP
                    IF ${side}_recorded -> name <> 'null'::jsonb THEN
                        ${side}_recorded := jsonb_set(${side}_recorded, ARRAY[name], to_jsonb(substr(${side}_recorded ->> name, 3)));
                    END IF;
                END LOOP;
            END IF;`;
}

/** Create the function refusing an update of an append-only table. */
function createRefusal(): string {
    return `CREATE OR REPLACE FUNCTION ${quote(`${LOG}_append`)}() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            RAISE EXCEPTION 'append-only table: %', TG_TABLE_NAME;
        END $$`;
}

/** Generate the trigger on a table's relation calling the shared PostgreSQL function. */
function postgresLogTriggers(target: string, description: ChangeDescription): string[] {
    // write the column lists as array literals
    const table = quote(target);

    // pass the retention, the column lists and the scope column when the table has one
    const parameters = [
        literal(description.retention),
        literal(array(description.key)),
        literal(array(description.columns)),
        literal(array(description.exact)),
        literal(array(description.binary)),
        ...(description.scope === undefined ? [] : [literal(description.scope)]),
    ];

    // log only the insertions of an append-only table, refusing its updates and leaving its deletions unlogged
    if (description.appendOnly === true) {
        return [
            createRefusal(),
            `CREATE TRIGGER ${quote("destack_change")}
                AFTER INSERT ON ${table}
                FOR EACH ROW
                EXECUTE FUNCTION ${quote(`${LOG}_record`)}(${parameters.join(", ")})`,
            `CREATE TRIGGER ${quote("destack_append")}
                BEFORE UPDATE ON ${table}
                FOR EACH ROW
                EXECUTE FUNCTION ${quote(`${LOG}_append`)}()`,
        ];
    }

    return [
        `CREATE TRIGGER ${quote("destack_change")}
            AFTER INSERT OR UPDATE OR DELETE ON ${table}
            FOR EACH ROW
            EXECUTE FUNCTION ${quote(`${LOG}_record`)}(${parameters.join(", ")})`,
    ];
}

/** Write column names as a PostgreSQL array literal. */
function array(names: readonly string[]): string {
    return `{${names.map((name) => `"${name.replaceAll('"', '\\"')}"`).join(",")}}`;
}
