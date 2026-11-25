import { canonicalize, present, schema, Version } from "@destack/schema";
import { Expression } from "../expression/expression.ts";
import { PackageVersion, type PackageId } from "@destack/package";
import { TABLE, type Table } from "../table/table.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import {
    type ConstraintDescription,
    describeColumn,
    describeTable,
    type IndexDescription,
    TableDescription,
} from "../table/description.ts";
import { ChangeDescription } from "../log/description.ts";
import { AggregateDescription } from "../aggregate/description.ts";
import { DependentDescription } from "../dependent/description.ts";
import { dialectSQL, inline, placed, sql } from "../sql/index.ts";
import { boundedName, qualify, relation } from "../table/namespace.ts";
import { describeLog, loggedKey, REPLICA_COLUMN } from "../log/schema.ts";
import { integer } from "../table/column.ts";
import { literal, quote } from "../dialect/quote.ts";
import { DatabaseError } from "../error/error.ts";

import { MultikeyIndexDescription } from "../multikey/description.ts";
import { expandMultikeyIndexes } from "../multikey/multikey.ts";

/** The SQL name of the applied state table. */
export const STATE = "__destack_state";

/** The table with each manager's last applied declaration digest. */
export const STAMP = "__destack_stamp";

/** The manager of a database's own tables. */
export const DATABASE_MANAGER = "database";

/** The column of the state and stamp tables with the manager that applied a row. */
const MANAGER_COLUMN = "manager";

/** A foreign key as the database stores it. */
type ForeignKeyDescription = Extract<ConstraintDescription, { readonly kind: "foreignKey" }>;

/** A managed table as declared or applied. */
export const TableState = schema.object({
    /** The package release declaring the table. */
    package: PackageVersion,
    /** The table's columns, keys, indexes and checks. */
    table: TableDescription,
    /** Whether the declaration replicates the table from the package that owns it. */
    isReplicated: schema.literal(true).exactOptional(),
    /** The columns the change triggers record, absent for unlogged tables. */
    log: ChangeDescription.exactOptional(),
    /** The table's multikey indexes. */
    multikeyIndexes: schema.array(MultikeyIndexDescription).exactOptional(),
    /** The aggregates the table declares, of its rows or of another table's into it. */
    aggregates: schema.array(AggregateDescription).exactOptional(),
    /** The rows of other tables referencing this table's rows. */
    dependents: schema.array(DependentDescription).exactOptional(),
    /** The SQL names of the text columns the full-text index reads. */
    fullText: schema.array(schema.string().min(1)).exactOptional(),
    /** The global indexes over the rows, with the SQL names of each row's identifier and scope. */
    globalIndexes: schema
        .array(
            schema.object({
                /** The index's identity. */
                name: schema.string().min(1),
                /** The index's declaration as canonical JSON. */
                declaration: schema.string().min(1),
                /** The SQL name of the column with each row's identifier. */
                id: schema.string().min(1),
                /** The SQL name of the column with each row's scope. */
                scope: schema.string().min(1),
                /** The SQL name of the outbox queueing rows for the home. */
                outbox: schema.string().min(1),
            }),
        )
        .exactOptional(),
    /** The predefined rows as SQL literals by column name, upserted in every database. */
    predefined: schema.array(schema.record(schema.string(), schema.string())).exactOptional(),
    /** The previous SQL names of the table and its columns. */
    moved: schema
        .object({
            /** The table's previous SQL name. */
            table: schema.string().min(1).exactOptional(),
            /** The previous SQL column names by current name. */
            columns: schema.record(schema.string(), schema.string()),
        })
        .exactOptional(),
    /** The renamed columns equal to their previous columns. */
    bridges: schema
        .array(
            schema.object({
                /** The previous column an older release writes. */
                from: schema.string().min(1),
                /** The renamed column the newest release writes. */
                to: schema.string().min(1),
            }),
        )
        .exactOptional(),
    /** The assignments converting each earlier row, by the edition each leads to and the column. */
    conversions: schema
        .record(
            schema.string().regex(/^[1-9]\d*$/u),
            schema.record(schema.string(), Expression.schema),
        )
        .exactOptional(),
});
/** A managed table as declared or applied. */
export type TableState = schema.Infer<typeof TableState>;

/** The tables a database declaration requires in every dialect. */
export const DatabaseState = Object.assign(
    schema.object({
        /** The declared table states per dialect. */
        tables: schema.object({
            /** The SQLite table states. */
            sqlite: schema.array(TableState),
            /** The PostgreSQL table states. */
            postgresql: schema.array(TableState),
        }),
    }),
    {
        /** Describe tables in every dialect. */
        of(tables: readonly Table[], options: DeclareOptions = {}): DatabaseState {
            return {
                tables: {
                    sqlite: declareState(tables, "sqlite", options),
                    postgresql: declareState(tables, "postgresql", options),
                },
            };
        },
    },
);
/** The tables a database declaration requires in every dialect. */
export type DatabaseState = schema.Infer<typeof DatabaseState>;

/** The options of a table declaration. */
export interface DeclareOptions {
    /** Whether the whole database is a replica, without foreign keys. */
    readonly isReplica?: boolean;
    /** The SQL names of replicated tables, which enforce their keys on the home's rows only. */
    readonly replicated?: ReadonlySet<string>;
    /** The packages whose tables to describe, placing the others' tables; all when absent. */
    readonly packages?: readonly PackageId[];
}

/** List the declared tables with an unapplied declaration. */
export function unappliedTables(
    applied: readonly TableState[],
    declared: readonly TableState[],
): string[] {
    const recorded = new Map(applied.map((state) => [state.table.name, state]));

    return declared
        .filter((state) => {
            const current = recorded.get(state.table.name);

            return current === undefined || !coversState(current, state);
        })
        .map((state) => state.table.name);
}

/** Describe a state's log retention and scope column. */
export function logOf(state: TableState): string {
    return canonicalize({
        retention: state.log?.retention,
        appendOnly: state.log?.appendOnly,
        scope: state.log?.scope,
    });
}

/** Report whether an applied state covers a declaration. */
export function coversState(applied: TableState, declared: TableState): boolean {
    // require the same or a newer release and the same log
    if (
        Version.compare(applied.package.version, declared.package.version) < 0 ||
        logOf(applied) !== logOf(declared) ||
        canonicalize(applied.multikeyIndexes ?? []) !==
            canonicalize(declared.multikeyIndexes ?? []) ||
        canonicalize(applied.aggregates ?? []) !== canonicalize(declared.aggregates ?? []) ||
        canonicalize(applied.dependents ?? []) !== canonicalize(declared.dependents ?? [])
    ) {
        return false;
    }

    // require every declared column
    const columns = new Map(applied.table.columns.map((column) => [column.name, column]));
    const hasColumns = declared.table.columns.every((column) => {
        const current = columns.get(column.name);

        return (
            current !== undefined &&
            (current.nullable || !column.nullable) &&
            canonicalize({ ...current, nullable: column.nullable }) === canonicalize(column)
        );
    });

    // require every declared constraint and index
    const entries = new Map(
        [...applied.table.constraints, ...applied.table.indexes].map((entry) => [
            entry.name,
            canonicalize(entry),
        ]),
    );
    const hasConstraints = [...declared.table.constraints, ...declared.table.indexes].every(
        (entry) => entries.get(entry.name) === canonicalize(entry),
    );

    // require every declared logged column
    const logged = new Set(applied.log?.columns ?? []);
    const hasLog = (declared.log?.columns ?? []).every((column) => logged.has(column));

    return hasColumns && hasConstraints && hasLog;
}

/** Describe declared tables in one dialect. */
export function declareState(
    tables: readonly Table[],
    dialect: Dialect,
    options: DeclareOptions = {},
): TableState[] {
    // add foreign keys to tables this database has and leave other references logical
    const isReplica = options.isReplica === true;
    const declared = expandMultikeyIndexes(tables);
    const local = new Set(isReplica ? [] : declared.map((table) => table[TABLE].sqlName));
    const replicated = options.replicated ?? new Set<string>();
    const { packages } = options;
    const included =
        packages === undefined
            ? declared
            : declared.filter((table) => packages.includes(table[TABLE].package.id));

    // attach each aggregate of a described table to it, a replica's too
    const named = new Set(declared.map((table) => table[TABLE].sqlName));
    const aggregates = describeAggregates(included, named, isReplica);

    return included.map((table) => {
        // describe the table with its log
        const definition = table[TABLE];
        const log = describeLog(table);
        const multikeyIndexes = definition.multikeyIndexes.map((each) => each.describe());
        const moved = describeMoves(table);
        const conversions = describeConversions(table);
        const predefined = describePredefined(table, dialect);
        const isReplicated = replicated.has(definition.sqlName);
        const database = { dialect, local, replicated };
        const dependents = isReplica
            ? []
            : [
                  ...describeDependents(table, tables, isReplicated),
                  ...declared.flatMap((source) => describeReferences(source, table, database)),
              ];
        const aggregated = aggregates.get(definition.sqlName);

        // keep a replicated table's keys unique among its home rows, leaving references logical
        const described = withinDatabase(describeTable(table, dialect), local, replicated);
        const kept = isReplicated ? asReplicated(described, dialect) : described;

        return {
            package: definition.package,
            table: isReplica ? asReplica(table, kept) : kept,
            ...(isReplicated ? { isReplicated: true as const } : {}),
            ...(log === undefined ? {} : { log }),
            ...(multikeyIndexes.length === 0 ? {} : { multikeyIndexes }),
            ...(aggregated === undefined ? {} : { aggregates: aggregated }),
            ...(dependents.length === 0 ? {} : { dependents }),
            ...(definition.fullText === undefined
                ? {}
                : { fullText: [...definition.fullText.columns] }),
            ...(moved === undefined ? {} : { moved }),
            ...(conversions === undefined ? {} : { conversions }),
            ...(predefined.length === 0 ? {} : { predefined }),
            ...(isReplica || isReplicated || definition.globalIndexes.length === 0
                ? {}
                : { globalIndexes: [...definition.globalIndexes] }),
        };
    });
}

/** Describe a declared table state as a database replicating the table stores it. */
export function replicatedState(state: TableState, dialect: Dialect): TableState {
    // keep a replicated state, else mark its replicas and leave its global indexes to its home
    if (state.isReplicated === true) {
        return state;
    }
    const { globalIndexes: _globalIndexes, ...kept } = state;

    return { ...kept, isReplicated: true, table: asReplicated(state.table, dialect) };
}

/** Qualify a state's relations with a namespace, preserving the SQL name its log records. */
export function namespaceState(state: TableState, namespace: string | undefined): TableState {
    return namespace === undefined
        ? state
        : renameState(state, (name) => relation(name, namespace));
}

/** Read a namespaced state's relations without their namespace, as its declaration had them. */
export function localState(state: TableState, namespace: string | undefined): TableState {
    // return a separate database's state as it is
    if (namespace === undefined) {
        return state;
    }

    // drop the namespace's prefix from each relation
    return renameState(state, (name) => localName(name, namespace));
}

/** Drop a namespace's prefix from a relation's name, leaving any other name as it is. */
export function localName(name: string, namespace: string | undefined): string {
    const prefix = relation("", namespace);

    return name.startsWith(prefix) ? name.slice(prefix.length) : name;
}

/** Rename the table, its indexes and every table the state refers to. */
function renameState(state: TableState, name: (relation: string) => string): TableState {
    const { table, aggregates, dependents, multikeyIndexes, moved } = state;

    return {
        ...state,
        table: {
            ...table,
            name: name(table.name),
            constraints: table.constraints.map((constraint) =>
                constraint.kind === "foreignKey"
                    ? { ...constraint, table: name(constraint.table) }
                    : constraint,
            ),
            indexes: table.indexes.map((index) => ({ ...index, name: name(index.name) })),
        },
        ...(aggregates === undefined
            ? {}
            : {
                  aggregates: aggregates.map((aggregate) => ({
                      ...aggregate,
                      table: name(aggregate.table),
                      source: name(aggregate.source),
                  })),
              }),
        ...(dependents === undefined
            ? {}
            : {
                  dependents: dependents.map((dependent) => ({
                      ...dependent,
                      table: name(dependent.table),
                      source: name(dependent.source),
                  })),
              }),
        ...(multikeyIndexes === undefined
            ? {}
            : {
                  multikeyIndexes: multikeyIndexes.map((multikey) => ({
                      ...multikey,
                      table: name(multikey.table),
                      entryTable: name(multikey.entryTable),
                  })),
              }),
        ...(moved?.table === undefined ? {} : { moved: { ...moved, table: name(moved.table) } }),
    };
}

/** Create the stamp table within a database's namespace. */
export function createStamp(namespace?: string): string {
    return `CREATE TABLE IF NOT EXISTS ${quote(relation(STAMP, namespace))} (
        manager TEXT PRIMARY KEY,
        digest TEXT NOT NULL
    )`;
}

/** Create the state table within a database's namespace. */
export function createState(namespace?: string): string {
    return `CREATE TABLE IF NOT EXISTS ${quote(relation(STATE, namespace))} (
        "table" TEXT PRIMARY KEY,
        manager TEXT NOT NULL,
        package_id TEXT NOT NULL,
        state TEXT NOT NULL,
        applied_at BIGINT NOT NULL
    )`;
}

/** Read a connected database's table names, within its namespace in a shared SQLite store. */
export async function readTables(database: DatabaseConnection): Promise<string[]> {
    return (await readListing(database)).tables;
}

/** A database's tables, and those of its state and stamp tables from before managers. */
export interface Listing {
    /** The table names, within the database's namespace. */
    readonly tables: string[];
    /** Its state and stamp tables without the manager column, by name. */
    readonly unmanaged: string[];
}

/** An applied table's name without the namespace's prefix, and whether it is a replica. */
export interface Replication {
    /** The table's name in its database's namespace. */
    readonly table: string;
    /** Whether the table replicates another database's table. */
    readonly isReplicated: boolean;
}

/** Read which applied tables are replicas, by name, without decoding their full states. */
export async function readReplication(database: DatabaseConnection): Promise<Replication[]> {
    // find the state table
    const tables = await readTables(database);
    if (!tables.includes(relation(STATE, database.state.namespace))) {
        return [];
    }

    // read each table's replica flag from its stored state
    const rows = await database.execute(
        dialectSQL({
            sqlite: sql`SELECT "table", CASE WHEN json_extract(state, '$.isReplicated') = 1 THEN 1 ELSE 0 END AS replicated FROM ${sql.relation(STATE)}`,
            postgresql: sql`SELECT "table", CASE WHEN (state::jsonb ->> 'isReplicated') = 'true' THEN 1 ELSE 0 END AS replicated FROM ${sql.relation(STATE)}`,
        }),
        schema.object({ table: schema.string(), replicated: schema.number() }),
    );
    const { namespace } = database.state;

    return rows.map((row) => ({
        table: localName(row.table, namespace),
        isReplicated: row.replicated === 1,
    }));
}

/** List a database's tables with the columns of its state and stamp tables, in one statement. */
export async function readListing(database: DatabaseConnection): Promise<Listing> {
    // list the tables whose names start with the namespace's prefix, every one outside a namespace
    const { namespace } = database.state;
    const prefix = relation("", namespace);
    const kept = [relation(STATE, namespace), relation(STAMP, namespace)];
    const [state, stamp] = kept;
    const rows = await database.execute(
        dialectSQL({
            sqlite: sql`SELECT name, CASE WHEN name IN (${state}, ${stamp}) THEN (
                    SELECT group_concat(described.name) FROM pragma_table_info(sqlite_schema.name) AS described
                ) END AS columns
                FROM sqlite_schema WHERE type = 'table' AND substr(name, 1, ${prefix.length}) = ${prefix}`,
            postgresql: sql`SELECT c.relname AS name, CASE WHEN c.relname IN (${state}, ${stamp}) THEN (
                    SELECT string_agg(a.attname, ',') FROM pg_attribute a
                    WHERE a.attrelid = c.oid AND a.attnum > 0 AND NOT a.attisdropped
                ) END AS columns
                FROM pg_depend d JOIN pg_class c ON c.oid = d.objid
                WHERE d.classid = 'pg_class'::regclass AND d.refclassid = 'pg_namespace'::regclass
                    AND d.refobjid = ${database.state.schema ?? null}::regnamespace AND c.relkind IN ('r', 'p')`,
        }),
        schema.object({ name: schema.string(), columns: schema.string().nullable() }),
    );

    // take a state or stamp table without the manager column as one from before managers
    const unmanaged = rows.filter(
        (row) => row.columns !== null && !row.columns.split(",").includes(MANAGER_COLUMN),
    );

    return { tables: rows.map((row) => row.name), unmanaged: unmanaged.map((row) => row.name) };
}

/** Give a database's state and stamp tables from before managers their current shape. */
export async function upgradeUnmanaged(
    database: DatabaseConnection,
    listing: Listing,
): Promise<void> {
    // record the database as the manager of every table its one declaration applied
    const { namespace } = database.state;
    const state = relation(STATE, namespace);
    const stamp = relation(STAMP, namespace);
    const guard = database.dialect === "postgresql" ? "IF NOT EXISTS " : "";
    if (listing.unmanaged.includes(state)) {
        await database.executeScript(
            `ALTER TABLE ${quote(state)} ADD COLUMN ${guard}${MANAGER_COLUMN} TEXT NOT NULL DEFAULT ${literal(DATABASE_MANAGER)}`,
        );
    }

    // drop its one stamp, which the next migration writes again for its manager
    if (listing.unmanaged.includes(stamp)) {
        await database.executeScript(`DROP TABLE IF EXISTS ${quote(stamp)}`);
    }
}

/** List the tables among some that have no rows, a column each in statements the engine fits. */
export async function readEmpty(
    database: DatabaseConnection,
    tables: readonly string[],
): Promise<Set<string>> {
    // ask as many tables per statement as the engine returns columns
    const empty = new Set<string>();
    const { columns } = database.driver.session.limits;
    for (let start = 0; start < tables.length; start += columns) {
        // ask each table of the chunk for its first row, a column each, as no compound select is needed
        const chunk = tables.slice(start, start + columns);
        const { schema: inSchema } = database.state;
        const checks = chunk.map(
            (name, index) =>
                `NOT EXISTS (SELECT 1 FROM ${placed(name, { schema: inSchema })}) AS ${quote(`t${index}`)}`,
        );
        const [row] = await database.execute(
            sql.raw(`SELECT ${checks.join(", ")}`),
            schema.record(schema.string(), schema.union([schema.boolean(), schema.number()])),
        );

        // keep the tables answering true, SQLite's 1 or PostgreSQL's true
        for (const [index, name] of chunk.entries()) {
            const answer = present(row, "the emptiness check's row")[`t${index}`];
            if (answer === true || answer === 1) {
                empty.add(name);
            }
        }
    }

    return empty;
}

/** A table's applied state and the manager whose plan applied it. */
export interface AppliedState {
    /** The manager that applied it. */
    readonly manager: string;
    /** The applied state. */
    readonly state: TableState;
}

/** The stamp of a database's last declaration: its digest, none before stamps, or none yet. */
export type Stamp =
    | { readonly kind: "stamped"; readonly digest: string }
    | { readonly kind: "unstamped" }
    | { readonly kind: "new" };

/** Read the stamp of the declaration a manager applied last, given the database's tables. */
export async function readStamp(
    database: DatabaseConnection,
    manager: string,
    tables: readonly string[],
): Promise<Stamp> {
    // read a database migrated before stamps, whose stamp table an applied plan creates, apart from a new one
    if (!tables.includes(relation(STAMP, database.state.namespace))) {
        return tables.includes(relation(STATE, database.state.namespace))
            ? { kind: "unstamped" }
            : { kind: "new" };
    }

    // read every manager's stamp
    const [row] = await database.execute(
        sql`SELECT digest FROM ${sql.relation(STAMP)} WHERE manager = ${manager}`,
        schema.object({ digest: schema.string() }),
    );

    return row === undefined ? { kind: "new" } : { kind: "stamped", digest: row.digest };
}

/** Record the stamp of a manager's applied declaration, creating a missing stamp table first. */
export async function writeStamp(
    database: DatabaseConnection,
    manager: string,
    digest: string,
    isUnstamped: boolean,
): Promise<void> {
    // create the table a database migrated before stamps lacks, then record the digest
    if (isUnstamped) {
        await database.executeScript(createStamp(database.state.namespace));
    }
    await database.run(
        sql`INSERT INTO ${sql.relation(STAMP)} (manager, digest) VALUES (${manager}, ${digest})
            ON CONFLICT (manager) DO UPDATE SET digest = excluded.digest`,
    );
}

/** Read every managed table's applied state, with its manager. */
export async function readState(database: DatabaseConnection): Promise<AppliedState[]> {
    // find the state table
    const tables = await readTables(database);
    if (!tables.includes(relation(STATE, database.state.namespace))) {
        return [];
    }

    // read each table's state in name order
    const rows = await database.execute(
        sql`SELECT "table", state, manager FROM ${sql.relation(STATE)} ORDER BY "table"`,
        schema.object({ table: schema.string(), state: schema.string(), manager: schema.string() }),
    );

    // decode each state, refusing one in a format this release no longer reads
    const states: AppliedState[] = [];
    for (const row of rows) {
        const parsed = TableState.safeParse(JSON.parse(row.state));
        if (!parsed.success) {
            const name = await readName(database);
            const detail = schema.prettifyError(parsed.error);
            throw DatabaseError.of(
                "MIGRATION_BROKEN",
                `the applied state of table '${row.table}' in database '${name}' has an unknown format: ${detail}`,
                { cause: parsed.error },
            );
        }
        states.push({ manager: row.manager, state: parsed.data });
    }

    return states;
}

/** Read the manager of every managed table, by the table's name without its namespace. */
export async function readManagers(database: DatabaseConnection): Promise<Map<string, string>> {
    // find the state table
    const { namespace } = database.state;
    const tables = await readTables(database);
    if (!tables.includes(relation(STATE, namespace))) {
        return new Map();
    }

    // read each table's manager, leaving its state unparsed
    const rows = await database.execute(
        sql`SELECT "table", manager FROM ${sql.relation(STATE)}`,
        schema.object({ table: schema.string(), manager: schema.string() }),
    );
    const prefix = namespace === undefined ? "" : relation("", namespace);

    return new Map(
        rows.map((row) => [
            row.table.startsWith(prefix) ? row.table.slice(prefix.length) : row.table,
            row.manager,
        ]),
    );
}

/** Read the database's name: the server's database and schema, or a SQLite store's namespace. */
export async function readName(database: DatabaseConnection): Promise<string> {
    // read a SQLite database's namespace in a shared store
    const { namespace } = database.state;
    if (database.dialect === "sqlite") {
        return namespace === undefined ? "sqlite" : `sqlite namespace ${namespace}`;
    }

    // read the server's database and schema
    const [row] = await database.execute(
        sql`SELECT current_database() || '.' || ${present(database.state.schema, "the database's schema")} AS name`,
        schema.object({ name: schema.string() }),
    );

    return present(row, "the current database").name;
}

/** Record the applied state of one table, its applied time past the previous one on every apply. */
export function writeState(
    state: TableState,
    manager: string,
    appliedAt: number,
    namespace?: string,
): string {
    // store the conversions, so a later build runs only the ones it adds
    const encoded = literal(JSON.stringify(state));
    const table = quote(relation(STATE, namespace));

    return `INSERT INTO ${table} ("table", manager, package_id, state, applied_at)
        VALUES (${literal(state.table.name)}, ${literal(manager)}, ${literal(state.package.id)}, ${encoded}, ${appliedAt})
        ON CONFLICT ("table") DO UPDATE SET manager = excluded.manager,
            package_id = excluded.package_id,
            state = excluded.state,
            applied_at = CASE WHEN excluded.applied_at > ${table}.applied_at
                THEN excluded.applied_at ELSE ${table}.applied_at + 1 END`;
}

/** Forget the applied state of a dropped table within a database's namespace. */
export function deleteState(table: string, namespace?: string): string {
    return `DELETE FROM ${quote(relation(STATE, namespace))} WHERE "table" = ${literal(table)}`;
}

/** Describe a table's previous names in SQL terms. */
function describeMoves(table: Table): TableState["moved"] {
    // map each moved column to its previous name
    const definition = table[TABLE];
    const columns = Object.fromEntries(
        Object.entries(definition.moved.columns ?? {}).flatMap(([property, previous]) =>
            previous === undefined ? [] : [[definition.column(property).definition.name, previous]],
        ),
    );
    if (definition.moved.table === undefined && Object.keys(columns).length === 0) {
        return undefined;
    }

    return {
        ...(definition.moved.table === undefined
            ? {}
            : { table: qualify(definition.package, definition.moved.table) }),
        columns,
    };
}

/** Render each predefined row as SQL literals of its stored columns, by column name. */
function describePredefined(table: Table, dialect: Dialect): Record<string, string>[] {
    const definition = table[TABLE];

    return definition.predefined.map((row) =>
        Object.fromEntries(
            Object.entries(definition.columns)
                .filter(([, column]) => column.definition.generated === undefined)
                .map(([property, column]) => {
                    // require every stored column's value
                    const value = row[property];
                    if (value === undefined) {
                        throw new TypeError(
                            `predefined row of '${definition.name}' has no '${property}' value`,
                        );
                    }

                    return [column.definition.name, inline(sql.param(value, column), dialect)];
                }),
        ),
    );
}

/** Describe each edition's conversion over the stored row, its declared columns by SQL name. */
function describeConversions(table: Table): TableState["conversions"] {
    // read each assignment's columns by SQL name, a name the table lacks as a stored column's
    const definition = table[TABLE];
    const releases = Object.entries(definition.convert);
    if (releases.length === 0) {
        return undefined;
    }
    const columnOf = (property: string) =>
        definition.columns[property]?.definition.name ?? property;

    return Object.fromEntries(
        releases.map(([release, conversion]) => [
            release,
            Object.fromEntries(
                Object.entries(conversion).flatMap(([property, expression]) =>
                    expression === undefined
                        ? []
                        : [
                              [
                                  definition.column(property).definition.name,
                                  Expression.rename(expression, columnOf),
                              ],
                          ],
                ),
            ),
        ]),
    );
}

/** Describe the aggregates the described tables declare, skipping those a replica lacks. */
function describeAggregates(
    described: readonly Table[],
    named: ReadonlySet<string>,
    isReplica: boolean,
): Map<string, AggregateDescription[]> {
    const aggregates = new Map<string, AggregateDescription[]>();
    for (const table of described) {
        for (const aggregate of table[TABLE].aggregates) {
            // require both tables in the database by name unless a replica lacks one
            const source = "from" in aggregate ? aggregate.from() : table;
            const holder = "into" in aggregate ? aggregate.into() : table;
            const definition = source[TABLE];
            const missing = [source, holder].find((entry) => !named.has(entry[TABLE].sqlName));
            if (missing !== undefined) {
                if (isReplica) {
                    continue;
                }
                throw new TypeError(
                    `aggregate of '${definition.name}' into '${holder[TABLE].name}' references undeclared table '${missing[TABLE].name}'`,
                );
            }

            // require a single-column key
            const [id, ...rest] = loggedKey(holder);
            if (id === undefined || rest.length > 0) {
                throw new TypeError(
                    `aggregate into '${holder[TABLE].name}' needs a single-column key`,
                );
            }

            // refer to every column by its SQL name, on the state of the table declaring it
            const declaring = table[TABLE].sqlName;
            const entries = aggregates.get(declaring) ?? [];
            const target = {
                table: holder[TABLE].sqlName,
                column: holder[TABLE].column(aggregate.column).definition.name,
                id: id.definition.name,
                source: definition.sqlName,
                key: source[TABLE].column(aggregate.key).definition.name,
                where: Object.entries(aggregate.where ?? {}).map(([name, value]) => ({
                    column: source[TABLE].column(name).definition.name,
                    value,
                })),
            };
            entries.push(
                aggregate.function === "count"
                    ? { ...target, function: "count" }
                    : {
                          ...target,
                          function: aggregate.function,
                          value: source[TABLE].column(aggregate.value).definition.name,
                      },
            );
            aggregates.set(declaring, entries);
        }
    }

    return aggregates;
}

/** Describe the rows referencing a table's rows that this database cascades. */
function describeDependents(
    table: Table,
    tables: readonly Table[],
    isReplicated: boolean,
): DependentDescription[] {
    // take a replicated table's dependents this database has, its declarer cascading the others
    const definition = table[TABLE];
    const kept = definition.dependents.filter(
        (dependent) => !isReplicated || tables.includes(dependent.from()),
    );

    return kept.map((dependent) => {
        // require the dependent table and a single-column key
        const source = dependent.from();
        if (!tables.includes(source)) {
            throw new TypeError(
                `dependents of '${definition.name}' reference undeclared table '${source[TABLE].name}'`,
            );
        }
        const [id, ...rest] = loggedKey(table);
        if (id === undefined || rest.length > 0) {
            throw new TypeError(`dependents of '${definition.name}' need a single-column key`);
        }

        return {
            table: definition.sqlName,
            source: source[TABLE].sqlName,
            keys: [
                {
                    column: source[TABLE].column(dependent.key).definition.name,
                    references: id.definition.name,
                },
            ],
            where: Object.entries(dependent.where ?? {}).map(([name, value]) => ({
                column: source[TABLE].column(name).definition.name,
                value,
            })),
            onDelete: dependent.onDelete,
        };
    });
}

/** Let a replica leave out the columns its log never records: sensitive ones. */
function asReplica(table: Table, described: TableDescription): TableDescription {
    const logged = new Set(
        Object.values(table[TABLE].logged).map((column) => column.definition.name),
    );

    return {
        ...described,
        columns: described.columns.map((column) =>
            logged.has(column.name) ? column : { ...column, nullable: true },
        ),
    };
}

/** Select a table's foreign keys to the database's tables, leaving replica references logical. */
function withinDatabase(
    table: TableDescription,
    local: ReadonlySet<string>,
    replicated: ReadonlySet<string>,
): TableDescription {
    return {
        ...table,
        constraints: table.constraints.filter(
            (constraint) =>
                constraint.kind !== "foreignKey" ||
                (local.has(constraint.table) && !replicated.has(constraint.table)),
        ),
    };
}

/** Describe a replicated table: replicas marked, keys unique among the rest, no foreign keys. */
function asReplicated(described: TableDescription, dialect: Dialect): TableDescription {
    // cover the rows the home wrote
    const home = `${quote(REPLICA_COLUMN)} = 0`;

    // require values in the home's rows alone, except in the primary key and the scope, which the log keys rows by
    const keyed = new Set([
        "scope",
        ...described.constraints.flatMap((constraint) =>
            constraint.kind === "primaryKey" ? constraint.columns : [],
        ),
    ]);
    const required = described.columns.filter(
        (column) => !column.nullable && !keyed.has(column.name),
    );
    const checks = required.map((column) => ({
        kind: "check" as const,
        name: boundedName(`${described.name}_${column.name}_home`),
        expression: `${quote(column.name)} IS NOT NULL OR ${quote(REPLICA_COLUMN)} = 1`,
    }));

    return {
        ...described,
        columns: [
            ...described.columns.map((column) =>
                keyed.has(column.name) ? column : { ...column, nullable: true },
            ),
            describeColumn(integer(REPLICA_COLUMN).notNull().default(0), dialect),
        ],
        constraints: [
            ...described.constraints.filter(
                (constraint) => constraint.kind !== "unique" && constraint.kind !== "foreignKey",
            ),
            ...checks,
        ],
        indexes: [...described.indexes, ...uniqueIndexes(described)].map((index) =>
            !index.unique
                ? index
                : {
                      ...index,
                      where: index.where === undefined ? home : `(${index.where}) AND ${home}`,
                  },
        ),
    };
}

/** Describe a replicated table's home rows referencing a local table as its dependents. */
function describeReferences(
    source: Table,
    target: Table,
    database: {
        /** The dialect the foreign keys are described in. */
        readonly dialect: Dialect;
        /** The SQL names of the database's tables. */
        readonly local: ReadonlySet<string>;
        /** The SQL names of the replicated tables. */
        readonly replicated: ReadonlySet<string>;
    },
): DependentDescription[] {
    // take a replicated table's foreign keys to the target, a local table whose rows all live here
    const { dialect, local, replicated } = database;
    const name = target[TABLE].sqlName;
    if (!replicated.has(source[TABLE].sqlName) || !local.has(name) || replicated.has(name)) {
        return [];
    }
    const keys = describeTable(source, dialect).constraints.filter(
        (constraint): constraint is ForeignKeyDescription =>
            constraint.kind === "foreignKey" && constraint.table === name,
    );

    // cascade or refuse the target's deletes into the home's rows, which replicas leave alone
    return keys.map((constraint) => ({
        table: name,
        source: source[TABLE].sqlName,
        keys: constraint.columns.map((column, index) => ({
            column,
            references: present(
                constraint.references[index],
                "the foreign key's referenced column",
            ),
        })),
        where: [{ column: REPLICA_COLUMN, value: 0 }],
        onDelete: deletion(constraint),
    }));
}

/** Read how deleting a referenced row treats a foreign key's rows: cascade or refuse. */
function deletion(constraint: ForeignKeyDescription): DependentDescription["onDelete"] {
    const { onDelete = "no action" } = constraint;

    // cascade
    if (onDelete === "cascade") {
        return "cascade";
    }
    // refuse for no action
    else if (onDelete === "no action") {
        return "restrict";
    }
    // reject an action that writes the referencing rows, which a trigger here does not
    else {
        throw new TypeError(
            `foreign key '${constraint.name}' is invalid: a replicated table deletes by cascade or no action, not '${onDelete}'`,
        );
    }
}

/** Describe a table's unique constraints as unique indexes over the same columns. */
function uniqueIndexes(described: TableDescription): IndexDescription[] {
    return described.constraints.flatMap((constraint) =>
        constraint.kind === "unique"
            ? [
                  {
                      name: constraint.name,
                      unique: true,
                      columns: constraint.columns.map((column) => ({ column })),
                  },
              ]
            : [],
    );
}
