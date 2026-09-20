import * as postgres from "drizzle-orm/pg-core";
import { SQL } from "drizzle-orm";
import { applyColumn, SchemaCompiler } from "../dialect/compiler.ts";
import { Column, type ColumnDefinition } from "../table/column.ts";
import { TABLE, type Table } from "../table/table.ts";
import type { TableConstraint } from "../table/constraint.ts";
import type { PostgresColumnType } from "drizzle-orm/pg-core/codecs";

/** Native PostgreSQL codecs used for ordinary and relational selections. */
const POSTGRES_CODECS: Record<ColumnDefinition["kind"], PostgresColumnType> = {
    text: "text",
    integer: "bigint:number",
    real: "float8",
    boolean: "bool",
    json: "jsonb",
    binary: "bytea",
    bigint: "bigint",
    decimal: "numeric",
    timestamp: "timestamptz",
};

/** Compile portable declarations into Postgres Drizzle tables. */
export class PostgresSchemaCompiler extends SchemaCompiler<"postgresql"> {
    /** Compile the declared tables for Postgres. */
    constructor(declarations: readonly Table[]) {
        super("postgresql", declarations);
    }

    /** Materialize PostgreSQL columns and defer table constraints. */
    protected compileTable(declaration: Table): postgres.PgTable {
        const definition = declaration[TABLE];
        const columns = Object.fromEntries(
            Object.entries(definition.columns).map(([property, column]) => {
                const definition = column.definition;
                const builder = postgres.customType<{ data: unknown; driverData: unknown }>({
                    dataType: () => definition.types.postgresql,
                    toDriver: (value) => definition.encode(value, "postgresql"),
                    fromDriver: (value) => definition.decode(value, "postgresql"),
                    codec: POSTGRES_CODECS[definition.kind],
                })(definition.name);
                applyColumn(builder, column, this.dialect);
                if (definition.generated) {
                    if (definition.generated.mode !== "stored") {
                        throw new TypeError(
                            "The PostgreSQL adapter requires stored generated columns.",
                        );
                    }
                    const generated = definition.generated.expression;
                    builder.generatedAlwaysAs(() =>
                        this.expression(
                            typeof generated === "function" ? generated() : generated,
                        )
                    );
                }

                return [property, builder];
            }),
        );

        return postgres.pgTable(
            definition.name,
            columns,
            () =>
                declaration.constraints(this.dialect).map((constraint) =>
                    this.compileConstraint(constraint)
                ),
        );
    }

    /** Translate one PostgreSQL constraint. */
    private compileConstraint(constraint: TableConstraint): postgres.PgTableExtraConfigValue {
        // materialized columns belong to the selected dialect
        const columns = (values: readonly Column[]) =>
            values.map((column) => this.column(column) as postgres.PgColumn);

        switch (constraint.kind) {
            case "check":
                return postgres.check(constraint.name, this.expression(constraint.expression));
            case "primaryKey":
                return postgres.primaryKey({
                    name: constraint.name,
                    columns: columns(constraint.columns) as [
                        postgres.PgColumn,
                        ...postgres.PgColumn[],
                    ],
                });
            case "unique":
                return postgres.unique(constraint.name).on(
                    ...columns(constraint.columns) as [postgres.PgColumn, ...postgres.PgColumn[]],
                );
            case "index": {
                const builder = constraint.unique
                    ? postgres.uniqueIndex(constraint.name)
                    : postgres.index(constraint.name);
                const indexed = constraint.columns.map((column) =>
                    column instanceof Column
                        ? this.column(column) as postgres.PgColumn
                        : this.expression(column)
                );
                const index = builder.on(
                    ...indexed as [postgres.PgColumn | SQL, ...(postgres.PgColumn | SQL)[]],
                );

                return constraint.predicate
                    ? index.where(this.expression(constraint.predicate))
                    : index;
            }
            case "foreignKey": {
                const key = postgres.foreignKey({
                    name: constraint.name,
                    columns: columns(constraint.columns) as [
                        postgres.PgColumn,
                        ...postgres.PgColumn[],
                    ],
                    foreignColumns: columns(constraint.foreignColumns) as [
                        postgres.PgColumn,
                        ...postgres.PgColumn[],
                    ],
                });
                if (constraint.actions.onDelete) key.onDelete(constraint.actions.onDelete);
                if (constraint.actions.onUpdate) key.onUpdate(constraint.actions.onUpdate);

                return key;
            }
        }
    }
}
