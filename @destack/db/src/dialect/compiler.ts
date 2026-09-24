import type * as drizzle from "drizzle-orm";
import { assertNever } from "../error/error.ts";
import { getTableColumns, Param, SQL, type SQLChunk } from "drizzle-orm";
import * as sqlite from "drizzle-orm/sqlite-core";
import * as postgres from "drizzle-orm/pg-core";
import { describeColumnSchema } from "../inspect/table.ts";
import type { Dialect } from "./dialect.ts";
import { Column } from "../table/column.ts";
import { TABLE, Table } from "../table/table.ts";
import { compileExpression } from "./expression.ts";
import type { NativeRelations, NativeTable } from "./table.ts";
import * as relation from "drizzle-orm/relations";
import type { TableRelations } from "../schema/relation.ts";

/** Compile portable declarations into native Drizzle tables, columns, and relations. */
export abstract class SchemaCompiler<Driver extends Dialect = Dialect> {
    /** The selected SQL dialect. */
    readonly dialect: Driver;
    /** Physical tables indexed by their declared SQL names. */
    readonly tables = new Map<string, sqlite.SQLiteTable | postgres.PgTable>();
    /** Physical columns indexed by their logical declarations. */
    readonly columns = new WeakMap<Column, drizzle.Column>();
    /** Physical tables indexed by exact logical declaration. */
    readonly declarations = new WeakMap<Table, sqlite.SQLiteTable | postgres.PgTable>();

    /** Materialize each declared table once for this dialect. */
    constructor(dialect: Driver, declarations: readonly Table[]) {
        this.dialect = dialect;

        // build tables before evaluating foreign keys and other deferred constraints
        for (const declaration of declarations) {
            const definition = declaration[TABLE];
            if (definition.source) {
                throw new TypeError(`Query aliases cannot declare SQL tables: ${definition.name}.`);
            }
            if (this.tables.has(definition.name)) {
                throw new TypeError(`Duplicate SQL table: ${definition.name}.`);
            }
            const physical = this.compileTable(declaration);
            this.tables.set(definition.name, physical);
            this.declarations.set(declaration, physical);

            // retain exact column associations for expression and foreign-key translation
            const columns = getTableColumns(physical);
            for (const [property, column] of Object.entries(definition.columns)) {
                Object.assign(columns[property], {
                    schema: column.definition.schema,
                    jsonSchema: describeColumnSchema(column.definition),
                    enumValues: column.definition.enumValues,
                });
                this.columns.set(column, columns[property]);
            }
        }
    }

    /** Materialize declarations for Drizzle's relational query builders. */
    relations<Definitions extends Record<string, TableRelations>>(
        definitions: Definitions,
    ): NativeRelations<Driver, Definitions> {
        // retain application table names while binding physical columns
        const tables: Record<string, sqlite.SQLiteTable | postgres.PgTable> = Object.fromEntries(
            Object.entries(definitions).map(([name, definition]) => [
                name,
                this.table(definition.table),
            ]),
        );
        const relationBuilders = relation.createRelationsHelper(tables);
        const columns = new Map<Column, relation.RelationsBuilderColumn<string>>();
        for (const [name, definition] of Object.entries(definitions)) {
            for (const [property, column] of Object.entries(definition.table[TABLE].columns)) {
                columns.set(column, relationBuilders[name][property]);
            }
        }

        // let Drizzle check and compile the full relation collection
        const declarations: relation.AnyRelationsBuilderConfig = {};
        for (const [name, definition] of Object.entries(definitions)) {
            declarations[name] = {};
            for (const [property, reference] of Object.entries(definition.relations)) {
                const from = bindRelationColumns(reference.from, columns);
                const to = bindRelationColumns(reference.to, columns);
                declarations[name][property] =
                    reference.cardinality === "one"
                        ? relationBuilders.one[reference.target]({
                              from,
                              to,
                              optional: reference.optional,
                          })
                        : relationBuilders.many[reference.target]({ from, to });
            }
        }

        return relation.buildRelations(tables, declarations) as NativeRelations<
            Driver,
            Definitions
        >;
    }

    /** Translate a logical expression to physical columns. */
    expression<Value>(expression: SQL<Value>): SQL<Value> {
        // register nested table aliases before resolving columns that precede their FROM clause
        compileExpression(expression, this.dialect, (chunk) => {
            if (chunk instanceof Table) {
                this.table(chunk);
            }

            return chunk;
        });

        return compileExpression(expression, this.dialect, (chunk) => this.chunk(chunk));
    }

    /** Find the physical column belonging to a logical declaration. */
    column(column: Column): drizzle.Column {
        const physical = this.columns.get(column);
        if (!physical) {
            throw new TypeError(
                `Undeclared SQL column: ${column.table}.${column.definition.name}.`,
            );
        }

        return physical;
    }

    /** Find the physical table belonging to a declaration. */
    table<Definition extends Table>(declaration: Definition): NativeTable<Driver, Definition> {
        // find the compiled table or its source alias
        let physical = this.declarations.get(declaration);
        const source = declaration[TABLE].source;

        // register query aliases without retaining them in the persisted schema
        if (!physical && source) {
            const original = this.table(source);
            if (this.dialect === "sqlite") {
                physical = sqlite.alias(original as sqlite.SQLiteTable, declaration[TABLE].name);
            } else if (this.dialect === "postgresql") {
                physical = postgres.alias(original as postgres.PgTable, declaration[TABLE].name);
            } else {
                assertNever(this.dialect);
            }
            this.declarations.set(declaration, physical);
            const columns = getTableColumns(physical);
            for (const [property, column] of Object.entries(declaration[TABLE].columns)) {
                this.columns.set(column, columns[property]);
            }
        }
        if (!physical) {
            throw new TypeError(`Undeclared SQL table: ${declaration[TABLE].name}.`);
        }

        return physical as NativeTable<Driver, Definition>;
    }

    /** Compile one table using the selected engine's Drizzle builders. */
    protected abstract compileTable(declaration: Table): sqlite.SQLiteTable | postgres.PgTable;

    /** Translate nested declaration expressions. */
    private chunk(chunk: SQLChunk): SQLChunk {
        if (chunk instanceof Column) {
            return this.column(chunk);
        }
        if (chunk instanceof Param && chunk.encoder instanceof Column) {
            return new Param(chunk.value, this.column(chunk.encoder));
        }
        if (chunk instanceof SQL) {
            const decoder = (chunk as SQL & { decoder: unknown }).decoder;
            if (decoder instanceof Column) {
                chunk.mapWith(this.column(decoder));
            }
        }

        return chunk;
    }
}

/** Bind a relation's ordered columns to Drizzle's table references. */
function bindRelationColumns(
    columns: readonly Column[],
    bindings: ReadonlyMap<Column, relation.RelationsBuilderColumn<string>>,
): [relation.RelationsBuilderColumn<string>, ...relation.RelationsBuilderColumn<string>[]] {
    const bound = columns.map((column) => {
        const binding = bindings.get(column);
        if (!binding) {
            throw new TypeError("relation column is absent from the declared tables");
        }

        return binding;
    });

    return bound as [
        relation.RelationsBuilderColumn<string>,
        ...relation.RelationsBuilderColumn<string>[],
    ];
}

/** The Drizzle configuration of a custom column. */
type CustomColumnConfiguration = {
    // oxlint-disable-next-line destack/no-sludge -- Drizzle configuration key
    dataType: "custom";
    // oxlint-disable-next-line destack/no-sludge -- Drizzle configuration key
    data: unknown;
    // oxlint-disable-next-line destack/prevent-abbreviations -- Drizzle configuration key
    driverParam: unknown;
};

/** Apply shared column constraints through Drizzle's column builder API. */
export function applyColumn(
    builder:
        | sqlite.SQLiteCustomColumnBuilder<CustomColumnConfiguration>
        | postgres.PgCustomColumnBuilder<CustomColumnConfiguration>,
    column: Column,
    dialect: Dialect,
): void {
    // retain database defaults and application defaults separately
    const definition = column.definition;
    if (!definition.nullable) {
        builder.notNull();
    }
    if (definition.primaryKey) {
        builder.primaryKey();
    }
    if (definition.unique) {
        builder.unique(definition.unique.name);
    }
    if (definition.default !== undefined) {
        const value =
            definition.default instanceof SQL
                ? compileExpression(definition.default, dialect)
                : definition.default;
        builder.default(value);
    }

    // select dialect SQL each time an application default is evaluated
    const runtimeDefault = definition.runtimeDefault;
    if (runtimeDefault) {
        builder.$defaultFn(() => {
            const value = runtimeDefault();

            return value instanceof SQL ? compileExpression(value, dialect) : value;
        });
    }

    // select dialect SQL each time an application update value is evaluated
    const runtimeUpdate = definition.runtimeUpdate;
    if (runtimeUpdate) {
        builder.$onUpdateFn(() => {
            const value = runtimeUpdate();

            return value instanceof SQL ? compileExpression(value, dialect) : value;
        });
    }
}
