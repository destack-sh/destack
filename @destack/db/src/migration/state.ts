import { canonicalize } from "@destack/schema/json";
import { sql } from "drizzle-orm";
import { defineSchema, schema } from "@destack/schema";
import { TABLE, type Table } from "../table/table.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import { TableDescription } from "../inspect/table.ts";
import { ChangeDescription } from "../inspect/log.ts";
import { TreeDescription } from "../inspect/tree.ts";
import { AggregateDescription } from "../inspect/aggregate.ts";
import { describeTable, inlineExpression } from "../inspect/describe.ts";
import { qualify } from "../table/namespace.ts";
import { describeLog, primaryKey } from "../log/trigger.ts";
import { assertNever } from "../error/error.ts";
import { expandTrees } from "../tree/tree.ts";
import { literal, quote } from "../dialect/quote.ts";

/** The SQL name of the table recording each managed table's applied state. */
export const STATE = "__destack_state";

/** A managed table as a release declares it or a database applied it. */
export const TableState = defineSchema(
    schema.object({
        /** The package owning the table. */
        packageId: schema.string().min(1),
        /** The version of the row shape, raised by each declared conversion. */
        version: schema.number().int().positive(),
        /** The table's columns, keys, indexes and checks. */
        table: TableDescription,
        /** The columns the change triggers record, absent for unlogged tables. */
        log: ChangeDescription.optional(),
        /** The ancestor index maintained over the table, absent without a tree. */
        tree: TreeDescription.optional(),
        /** The aggregates of this table's rows other tables hold, kept current by triggers on this table. */
        aggregates: schema.array(AggregateDescription).optional(),
        /** The table's and its columns' previous SQL names, declared rather than applied. */
        moved: schema
            .object({
                /** The table's previous SQL name. */
                table: schema.string().min(1).optional(),
                /** Previous SQL column names indexed by current SQL column name. */
                columns: schema.record(schema.string(), schema.string()),
            })
            .optional(),
        /** Columns kept equal to the columns they were renamed from, while an older release still uses those. */
        bridges: schema
            .array(
                schema.object({
                    /** The previous column an older release writes. */
                    from: schema.string().min(1),
                    /** The renamed column the newest release writes. */
                    to: schema.string().min(1),
                }),
            )
            .optional(),
        /** Column assignments converting rows to each version above the first, declared rather than applied. */
        conversions: schema
            .record(schema.string(), schema.record(schema.string(), schema.string()))
            .optional(),
    }),
);
/** A managed table as a release declares it or a database applied it. */
export type TableState = schema.Infer<typeof TableState>;

/** The tables a database declaration requires in every dialect: its desired state. */
export const DatabaseState = defineSchema(
    schema.object({
        /** The declared table states per dialect. */
        tables: schema.object({
            /** The SQLite table states. */
            sqlite: schema.array(TableState),
            /** The PostgreSQL table states. */
            postgresql: schema.array(TableState),
        }),
    }),
);
/** The tables a database declaration requires in every dialect: its desired state. */
export type DatabaseState = schema.Infer<typeof DatabaseState>;

/** How to declare tables in a database. */
export interface DeclareOptions {
    /**
     * Whether the database holds a partial copy of the tables' rows.
     *
     * A copy keeps no foreign keys and no tree index, since the rows they refer to may stay at the source.
     */
    readonly isReplica?: boolean;
}

/** Name the declared tables whose applied state does not hold their declaration. */
export function unappliedTables(
    applied: readonly TableState[],
    declared: readonly TableState[],
): string[] {
    const recorded = new Map(applied.map((state) => [state.table.name, state]));

    return declared
        .filter((state) => {
            const current = recorded.get(state.table.name);

            return current === undefined || !holdsState(current, state);
        })
        .map((state) => state.table.name);
}

/** Report whether an applied state holds a declaration, as it does for every release it was merged from. */
export function holdsState(applied: TableState, declared: TableState): boolean {
    // require the same row shape version or a newer one, and the same log and tree
    const log = (state: TableState) =>
        canonicalize({ tier: state.log?.tier, route: state.log?.route });
    if (
        applied.version < declared.version ||
        log(applied) !== log(declared) ||
        canonicalize(applied.tree ?? null) !== canonicalize(declared.tree ?? null) ||
        canonicalize(applied.aggregates ?? []) !== canonicalize(declared.aggregates ?? [])
    ) {
        return false;
    }

    // require every declared column, allowing the applied one to accept NULL
    const columns = new Map(applied.table.columns.map((column) => [column.name, column]));
    const hasColumns = declared.table.columns.every((column) => {
        const current = columns.get(column.name);

        return (
            current !== undefined &&
            (current.nullable || !column.nullable) &&
            canonicalize({ ...current, nullable: column.nullable }) === canonicalize(column)
        );
    });

    // require every declared constraint and index under its name
    const entries = new Map(
        [...applied.table.constraints, ...applied.table.indexes].map((entry) => [
            entry.name,
            canonicalize(entry),
        ]),
    );
    const hasParts = [...declared.table.constraints, ...declared.table.indexes].every(
        (entry) => entries.get(entry.name) === canonicalize(entry),
    );

    // require the log to record every column the declaration records
    const logged = new Set(applied.log?.columns ?? []);
    const hasLog = (declared.log?.columns ?? []).every((column) => logged.has(column));

    return hasColumns && hasParts && hasLog;
}

/** Describe declared tables, their trees' index tables included, as data in one dialect. */
export function declareState(
    tables: readonly Table[],
    dialect: Dialect,
    options: DeclareOptions = {},
): TableState[] {
    // attach each aggregate to the table whose rows it aggregates
    const aggregates = describeAggregates(tables, options.isReplica ?? false);

    return (options.isReplica ? tables : expandTrees(tables)).map((table) => {
        // describe the table with its log recording and tree
        const definition = table[TABLE];
        const log = describeLog(table);
        const tree = options.isReplica ? undefined : definition.tree?.describe();
        const moved = describeMoves(table);
        const conversions = describeConversions(table, dialect);

        return {
            packageId: definition.package.id,
            version: definition.version,
            table: options.isReplica
                ? withoutReferences(describeTable(table, dialect))
                : describeTable(table, dialect),
            ...(log === undefined ? {} : { log }),
            ...(tree === undefined ? {} : { tree }),
            ...(aggregates.has(definition.sqlName)
                ? { aggregates: aggregates.get(definition.sqlName)! }
                : {}),
            ...(moved === undefined ? {} : { moved }),
            ...(conversions === undefined ? {} : { conversions }),
        };
    });
}

/** Create the state table, portable across dialects. */
export function createState(): string {
    return `CREATE TABLE IF NOT EXISTS "${STATE}" (
        "table" TEXT PRIMARY KEY,
        package_id TEXT NOT NULL,
        version INTEGER NOT NULL,
        state TEXT NOT NULL,
        applied_at BIGINT NOT NULL
    )`;
}

/** Read the names of the tables in a connected database's current schema. */
export async function readTables(database: DatabaseConnection): Promise<string[]> {
    const native = database.driver.native;
    const rows =
        native.dialect === "sqlite"
            ? await database.execute<{ name: string }>(
                  sql`SELECT name FROM sqlite_schema WHERE type = 'table'`,
              )
            : native.dialect === "postgresql"
              ? await database.execute<{ name: string }>(
                    sql`SELECT tablename AS name FROM pg_tables WHERE schemaname = current_schema()`,
                )
              : assertNever(native);

    return rows.map((row) => row.name);
}

/** Read the applied state of every managed table, empty before the first plan applied. */
export async function readState(database: DatabaseConnection): Promise<TableState[]> {
    // find the state table in the connection's schema
    if (!(await readTables(database)).includes(STATE)) {
        return [];
    }

    // decode each recorded table in name order
    const rows = await database.execute<{ state: string }>(
        sql`SELECT state FROM ${sql.identifier(STATE)} ORDER BY "table"`,
    );

    return rows.map((row) => TableState.parse(JSON.parse(row.state)));
}

/** Record the applied state of one table. */
export function writeState(state: TableState, appliedAt: number): string {
    // record the table without its moves and conversions
    const { moved: _moved, conversions: _conversions, ...applied } = state;
    const encoded = literal(JSON.stringify(applied));

    return `INSERT INTO ${quote(STATE)} ("table", package_id, version, state, applied_at)
        VALUES (${literal(state.table.name)}, ${literal(state.packageId)}, ${state.version}, ${encoded}, ${appliedAt})
        ON CONFLICT ("table") DO UPDATE SET package_id = excluded.package_id,
            version = excluded.version, state = excluded.state, applied_at = excluded.applied_at`;
}

/** Forget the applied state of a dropped table. */
export function deleteState(table: string): string {
    return `DELETE FROM ${quote(STATE)} WHERE "table" = ${literal(table)}`;
}

/** Describe a table's declared previous names in SQL terms. */
function describeMoves(table: Table): TableState["moved"] {
    // map each moved property's current SQL column to its previous name
    const definition = table[TABLE];
    const columns = Object.fromEntries(
        Object.entries(definition.moved.columns ?? {}).map(([property, previous]) => [
            definition.columns[property]!.definition.name,
            previous,
        ]),
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

/** Render each version's row conversion as SQL column assignments in one dialect. */
function describeConversions(table: Table, dialect: Dialect): TableState["conversions"] {
    // evaluate each conversion over the table's columns and inline its expressions
    const definition = table[TABLE];
    const versions = Object.entries(definition.convert);
    if (versions.length === 0) {
        return undefined;
    }

    return Object.fromEntries(
        versions.map(([version, convert]) => [
            version,
            Object.fromEntries(
                Object.entries(convert(definition.columns)).map(([property, value]) => [
                    definition.columns[property]!.definition.name,
                    inlineExpression(value, dialect),
                ]),
            ),
        ]),
    );
}

/** Describe the aggregates tables hold, by the table each aggregates; a replica skips holders it lacks, whose values arrive copied. */
function describeAggregates(
    tables: readonly Table[],
    isReplica: boolean,
): Map<string, AggregateDescription[]> {
    const described = new Map<string, AggregateDescription[]>();
    for (const source of tables) {
        const definition = source[TABLE];
        for (const aggregate of definition.aggregates) {
            // require the holding table among the declared ones, and a single-column key
            const holder = aggregate.into();
            if (!tables.includes(holder)) {
                if (isReplica) {
                    continue;
                }
                throw new TypeError(
                    `aggregate of ${definition.name} fills undeclared table ${holder[TABLE].name}`,
                );
            }
            const [id, ...rest] = primaryKey(holder);
            if (id === undefined || rest.length > 0) {
                throw new TypeError(
                    `aggregate into ${holder[TABLE].name} needs a single-column key`,
                );
            }

            // name every column by its SQL name
            const entries = described.get(definition.sqlName) ?? [];
            entries.push({
                table: holder[TABLE].sqlName,
                column: sqlColumn(holder, aggregate.column, source),
                id: id.definition.name,
                source: definition.sqlName,
                key: sqlColumn(source, aggregate.key, source),
                function: aggregate.function,
                ...(aggregate.value === undefined
                    ? {}
                    : { value: sqlColumn(source, aggregate.value, source) }),
                where: Object.entries(aggregate.where ?? {}).map(([name, value]) => ({
                    column: sqlColumn(source, name, source),
                    value,
                })),
            });
            described.set(definition.sqlName, entries);
        }
    }

    return described;
}

/** Leave out a table's foreign keys, which a partial copy cannot keep. */
function withoutReferences(table: TableDescription): TableDescription {
    return {
        ...table,
        constraints: table.constraints.filter((constraint) => constraint.kind !== "foreignKey"),
    };
}

/** Name a column an aggregate of a source table reads or fills by its SQL name. */
function sqlColumn(table: Table, name: string, source: Table): string {
    const column = table[TABLE].columns[name];
    if (!column) {
        throw new TypeError(`aggregate of ${source[TABLE].name} names no column ${name}`);
    }

    return column.definition.name;
}
