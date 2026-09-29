import { canonicalize } from "@destack/schema/json";
import { sql } from "drizzle-orm";
import { defineSchema, schema } from "@destack/schema";
import { Package } from "@destack/package";
import { TABLE, type Table } from "../table/table.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import { TableDescription } from "../inspect/table.ts";
import { ChangeDescription } from "../inspect/log.ts";
import { TreeDescription } from "../inspect/tree.ts";
import { AggregateDescription } from "../inspect/aggregate.ts";
import { DependentDescription } from "../inspect/dependent.ts";
import { describeTable, inlineExpression } from "../inspect/describe.ts";
import { qualify } from "../table/namespace.ts";
import { describeLog, primaryKey } from "../log/schema.ts";
import { assertNever } from "../error/error.ts";
import { expandTrees } from "../tree/tree.ts";
import { literal, quote } from "../dialect/quote.ts";

/** The SQL name of the applied state table. */
export const STATE = "__destack_state";

/** A managed table as declared or applied. */
export const TableState = defineSchema(
    schema.object({
        /** The package release declaring the table. */
        package: Package,
        /** The row shape version, raised by each conversion. */
        version: schema.number().int().positive(),
        /** The table's columns, keys, indexes and checks. */
        table: TableDescription,
        /** The columns the change triggers record, absent for unlogged tables. */
        log: ChangeDescription.optional(),
        /** The ancestor index maintained over the table, absent without a tree. */
        tree: TreeDescription.optional(),
        /** The aggregates of this table's rows that other tables hold. */
        aggregates: schema.array(AggregateDescription).optional(),
        /** The rows of other tables referencing this table's rows. */
        dependents: schema.array(DependentDescription).optional(),
        /** The previous SQL names of the table and its columns. */
        moved: schema
            .object({
                /** The table's previous SQL name. */
                table: schema.string().min(1).optional(),
                /** The previous SQL column names by current name. */
                columns: schema.record(schema.string(), schema.string()),
            })
            .optional(),
        /** The renamed columns kept equal to their previous columns. */
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
        /** The column assignments converting rows to each version above the first. */
        conversions: schema
            .record(schema.string(), schema.record(schema.string(), schema.string()))
            .optional(),
    }),
);
/** A managed table as declared or applied. */
export type TableState = schema.Infer<typeof TableState>;

/** The tables a database declaration requires in every dialect. */
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
/** The tables a database declaration requires in every dialect. */
export type DatabaseState = schema.Infer<typeof DatabaseState>;

/** The options of a table declaration. */
export interface DeclareOptions {
    /** Whether the database holds a partial copy, without foreign keys and tree indexes. */
    readonly isReplica?: boolean;
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

            return current === undefined || !holdsState(current, state);
        })
        .map((state) => state.table.name);
}

/** Describe a state's log retention and scope column. */
export function logOf(state: TableState): string {
    return canonicalize({ retention: state.log?.retention, scope: state.log?.scope });
}

/** Report whether an applied state holds a declaration. */
export function holdsState(applied: TableState, declared: TableState): boolean {
    // require the same or a newer version and the same log and tree
    if (
        applied.version < declared.version ||
        logOf(applied) !== logOf(declared) ||
        canonicalize(applied.tree ?? null) !== canonicalize(declared.tree ?? null) ||
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
    const hasParts = [...declared.table.constraints, ...declared.table.indexes].every(
        (entry) => entries.get(entry.name) === canonicalize(entry),
    );

    // require every declared logged column
    const logged = new Set(applied.log?.columns ?? []);
    const hasLog = (declared.log?.columns ?? []).every((column) => logged.has(column));

    return hasColumns && hasParts && hasLog;
}

/** Describe declared tables in one dialect. */
export function declareState(
    tables: readonly Table[],
    dialect: Dialect,
    options: DeclareOptions = {},
): TableState[] {
    // attach each aggregate to its aggregated table
    const aggregates = describeAggregates(tables, options.isReplica ?? false);

    // keep foreign keys to tables this database holds and leave other references logical
    const declared = options.isReplica ? tables : expandTrees(tables);
    const held = new Set(options.isReplica ? [] : declared.map((table) => table[TABLE].sqlName));

    return declared.map((table) => {
        // describe the table with its log and tree
        const definition = table[TABLE];
        const log = describeLog(table);
        const tree = options.isReplica ? undefined : definition.tree?.describe();
        const moved = describeMoves(table);
        const conversions = describeConversions(table, dialect);
        const dependents = options.isReplica ? [] : describeDependents(table, tables);

        return {
            package: definition.package,
            version: definition.version,
            table: options.isReplica
                ? replicated(table, withinDatabase(describeTable(table, dialect), held))
                : withinDatabase(describeTable(table, dialect), held),
            ...(log === undefined ? {} : { log }),
            ...(tree === undefined ? {} : { tree }),
            ...(aggregates.has(definition.sqlName)
                ? { aggregates: aggregates.get(definition.sqlName)! }
                : {}),
            ...(dependents.length === 0 ? {} : { dependents }),
            ...(moved === undefined ? {} : { moved }),
            ...(conversions === undefined ? {} : { conversions }),
        };
    });
}

/** Create the state table. */
export function createState(): string {
    return `CREATE TABLE IF NOT EXISTS "${STATE}" (
        "table" TEXT PRIMARY KEY,
        package_id TEXT NOT NULL,
        version INTEGER NOT NULL,
        state TEXT NOT NULL,
        applied_at BIGINT NOT NULL
    )`;
}

/** Read the table names of a connected database. */
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

/** Read every managed table's applied state. */
export async function readState(database: DatabaseConnection): Promise<TableState[]> {
    // find the state table
    if (!(await readTables(database)).includes(STATE)) {
        return [];
    }

    // decode each table in name order
    const rows = await database.execute<{ state: string }>(
        sql`SELECT state FROM ${sql.identifier(STATE)} ORDER BY "table"`,
    );

    return rows.map((row) => TableState.parse(JSON.parse(row.state)));
}

/** Record the applied state of one table. */
export function writeState(state: TableState, appliedAt: number): string {
    // drop the moves and conversions
    const { moved: _moved, conversions: _conversions, ...applied } = state;
    const encoded = literal(JSON.stringify(applied));

    return `INSERT INTO ${quote(STATE)} ("table", package_id, version, state, applied_at)
        VALUES (${literal(state.table.name)}, ${literal(state.package.id)}, ${state.version}, ${encoded}, ${appliedAt})
        ON CONFLICT ("table") DO UPDATE SET package_id = excluded.package_id,
            version = excluded.version, state = excluded.state, applied_at = excluded.applied_at`;
}

/** Forget the applied state of a dropped table. */
export function deleteState(table: string): string {
    return `DELETE FROM ${quote(STATE)} WHERE "table" = ${literal(table)}`;
}

/** Describe a table's previous names in SQL terms. */
function describeMoves(table: Table): TableState["moved"] {
    // map each moved column to its previous name
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

/** Render each version's conversion as SQL assignments. */
function describeConversions(table: Table, dialect: Dialect): TableState["conversions"] {
    // evaluate and inline each conversion
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

/** Describe the aggregates by aggregated table, skipping those a replica lacks. */
function describeAggregates(
    tables: readonly Table[],
    isReplica: boolean,
): Map<string, AggregateDescription[]> {
    const described = new Map<string, AggregateDescription[]>();
    for (const table of tables) {
        for (const aggregate of table[TABLE].aggregates) {
            // require both tables unless a replica lacks one
            const source = "from" in aggregate ? aggregate.from() : table;
            const holder = "into" in aggregate ? aggregate.into() : table;
            const definition = source[TABLE];
            const missing = [source, holder].find((entry) => !tables.includes(entry));
            if (missing !== undefined) {
                if (isReplica) {
                    continue;
                }
                throw new TypeError(
                    `aggregate of ${definition.name} into ${holder[TABLE].name} names undeclared table ${missing[TABLE].name}`,
                );
            }

            // require a single-column key
            const [id, ...rest] = primaryKey(holder);
            if (id === undefined || rest.length > 0) {
                throw new TypeError(
                    `aggregate into ${holder[TABLE].name} needs a single-column key`,
                );
            }

            // name every column by SQL name
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

/** Describe the rows referencing a table's rows. */
function describeDependents(table: Table, tables: readonly Table[]): DependentDescription[] {
    const definition = table[TABLE];

    return definition.dependents.map((dependent) => {
        // require the dependent table and a single-column key
        const source = dependent.from();
        if (!tables.includes(source)) {
            throw new TypeError(
                `dependents of ${definition.name} name undeclared table ${source[TABLE].name}`,
            );
        }
        const [id, ...rest] = primaryKey(table);
        if (id === undefined || rest.length > 0) {
            throw new TypeError(`dependents of ${definition.name} need a single-column key`);
        }

        return {
            table: definition.sqlName,
            id: id.definition.name,
            source: source[TABLE].sqlName,
            key: sqlColumn(source, dependent.key, source),
            where: Object.entries(dependent.where ?? {}).map(([name, value]) => ({
                column: sqlColumn(source, name, source),
                value,
            })),
            onDelete: dependent.onDelete,
        };
    });
}

/** Let a replica's copy leave out the columns its log never carries: binary and sensitive ones. */
function replicated(table: Table, described: TableDescription): TableDescription {
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

/** Keep only a table's foreign keys to held tables. */
function withinDatabase(table: TableDescription, held: ReadonlySet<string>): TableDescription {
    return {
        ...table,
        constraints: table.constraints.filter(
            (constraint) => constraint.kind !== "foreignKey" || held.has(constraint.table),
        ),
    };
}

/** Name a column of an aggregate by its SQL name. */
function sqlColumn(table: Table, name: string, source: Table): string {
    const column = table[TABLE].columns[name];
    if (!column) {
        throw new TypeError(`aggregate of ${source[TABLE].name} names no column ${name}`);
    }

    return column.definition.name;
}
