import * as sqlite from "drizzle-orm/sqlite-core";
import { SQL } from "drizzle-orm";
import { applyColumn, SchemaCompiler } from "../dialect/compiler.ts";
import { Column } from "../table/column.ts";
import { TABLE, type Table } from "../table/table.ts";
import type { TableConstraint } from "../table/constraint.ts";

/** Compile portable declarations into SQLite Drizzle tables. */
export class SQLiteSchemaCompiler extends SchemaCompiler<"sqlite"> {
    /** Compile the declared tables for SQLite. */
    constructor(declarations: readonly Table[]) {
        super("sqlite", declarations);
    }

    /** Materialize SQLite columns and defer table constraints. */
    protected compileTable(declaration: Table): sqlite.SQLiteTable {
        const definition = declaration[TABLE];
        const columns = Object.fromEntries(
            Object.entries(definition.columns).map(([property, column]) => {
                // build a custom column from the definition's codec
                const definition = column.definition;
                const builder = sqlite.customType<{ data: unknown; driverData: unknown }>({
                    dataType: () => definition.types.sqlite,
                    toDriver: (value) => definition.encode(value, "sqlite"),
                    fromDriver: (value) => definition.decode(value, "sqlite"),
                    fromJson: (value: unknown) =>
                        definition.decode(
                            definition.kind === "binary" && typeof value === "string"
                                ? Uint8Array.fromHex(value)
                                : value,
                            "sqlite",
                        ),
                    forJsonSelect:
                        definition.kind === "bigint"
                            ? (identifier, sql) => sql`cast(${identifier} as text)`
                            : undefined,
                })(definition.name);
                applyColumn(builder, column, this.dialect);
                if (definition.generated) {
                    const generated = definition.generated.expression;
                    builder.generatedAlwaysAs(
                        () =>
                            this.expression(
                                typeof generated === "function" ? generated() : generated,
                            ),
                        { mode: definition.generated.mode },
                    );
                }

                return [property, builder];
            }),
        );

        return sqlite.sqliteTable(definition.name, columns, () =>
            declaration
                .constraints(this.dialect)
                .map((constraint) => this.compileConstraint(constraint)),
        );
    }

    /** Translate one SQLite constraint. */
    private compileConstraint(constraint: TableConstraint): sqlite.SQLiteTableExtraConfigValue {
        // materialized columns belong to the selected dialect
        const columns = (values: readonly Column[]) =>
            values.map((column) => this.column(column) as sqlite.SQLiteColumn);

        switch (constraint.kind) {
            case "check":
                return sqlite.check(constraint.name, this.expression(constraint.expression));
            case "primaryKey":
                return sqlite.primaryKey({
                    name: constraint.name,
                    columns: columns(constraint.columns) as [
                        sqlite.SQLiteColumn,
                        ...sqlite.SQLiteColumn[],
                    ],
                });
            case "unique":
                return sqlite
                    .unique(constraint.name)
                    .on(
                        ...(columns(constraint.columns) as [
                            sqlite.SQLiteColumn,
                            ...sqlite.SQLiteColumn[],
                        ]),
                    );
            case "index": {
                const builder = constraint.unique
                    ? sqlite.uniqueIndex(constraint.name)
                    : sqlite.index(constraint.name);
                const indexed = constraint.columns.map((column) =>
                    column instanceof Column
                        ? (this.column(column) as sqlite.SQLiteColumn)
                        : this.expression(column),
                );
                const index = builder.on(
                    ...(indexed as [sqlite.SQLiteColumn | SQL, ...(sqlite.SQLiteColumn | SQL)[]]),
                );

                return constraint.predicate
                    ? index.where(this.expression(constraint.predicate))
                    : index;
            }
            case "foreignKey": {
                const key = sqlite.foreignKey({
                    name: constraint.name,
                    columns: columns(constraint.columns) as [
                        sqlite.SQLiteColumn,
                        ...sqlite.SQLiteColumn[],
                    ],
                    foreignColumns: columns(constraint.foreignColumns) as [
                        sqlite.SQLiteColumn,
                        ...sqlite.SQLiteColumn[],
                    ],
                });
                if (constraint.actions.onDelete) {
                    key.onDelete(constraint.actions.onDelete);
                }
                if (constraint.actions.onUpdate) {
                    key.onUpdate(constraint.actions.onUpdate);
                }

                return key;
            }
        }
    }
}
