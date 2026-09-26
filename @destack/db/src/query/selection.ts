import { SQL } from "drizzle-orm";
import * as drizzle from "drizzle-orm";
import { Column } from "../table/column.ts";
import { type Select, TABLE, Table } from "../table/table.ts";
import type { SchemaCompiler } from "../dialect/compiler.ts";
import type { DrizzleSelection } from "../dialect/drizzle.ts";

/** The source qualifier retained by inferred subquery fields. */
export declare const SOURCE: unique symbol;

/** Fields selected from tables, expressions, or nested groups. */
export interface Selection {
    /** A selected field or nested group. */
    readonly [property: string]: Column | drizzle.Column | SQL | SQL.Aliased | Table | Selection;
}

/** The application record produced by a selection. */
export type SelectionResult<Fields extends Selection, NullableTables extends string = never> = {
    [Property in keyof Fields]: Fields[Property] extends Column<
        infer Value,
        infer Required,
        boolean,
        infer Name
    >
        ? Name extends NullableTables
            ? Value | null
            : Required extends true
              ? Value
              : Value | null
        : Fields[Property] extends drizzle.Column
          ? Fields[Property]["_"]["data"]
          : Fields[Property] extends SQL<infer Value> | SQL.Aliased<infer Value>
            ? Fields[Property] extends { readonly [SOURCE]: infer Name }
                ? Name extends NullableTables
                    ? Value | null
                    : Value
                : Value
            : Fields[Property] extends Table
              ? Fields[Property][typeof TABLE]["name"] extends NullableTables
                  ? Select<Fields[Property]> | null
                  : Select<Fields[Property]>
              : Fields[Property] extends Selection
                ?
                      | SelectionResult<Fields[Property], NullableTables>
                      | NullableSelection<Fields[Property], NullableTables>
                : never;
};

/** Tables referenced directly by a nested selection. */
type SelectionTables<Fields extends Selection> = {
    [Property in keyof Fields]: Fields[Property] extends Column<
        unknown,
        boolean,
        boolean,
        infer Name
    >
        ? Name
        : Fields[Property] extends { readonly [SOURCE]: infer Name }
          ? Name
          : never;
}[keyof Fields];

/** A nested selection becomes NULL only when it belongs to one nullable joined table. */
type NullableSelection<
    Fields extends Selection,
    NullableTables extends string,
    Names = SelectionTables<Fields>,
    Name = Names,
> = Name extends NullableTables ? ([Names] extends [Name] ? null : never) : never;

/** Translate selected declarations while retaining property names and nesting. */
export function selectFields(fields: Selection, compiler: SchemaCompiler): DrizzleSelection {
    const selection: DrizzleSelection = {};

    // preserve SQL objects so their result decoders and aliases remain intact
    for (const [property, field] of Object.entries(fields)) {
        // translate a logical column
        if (field instanceof Column) {
            selection[property] = compiler.column(field);
        }
        // keep a native column
        else if (field instanceof drizzle.Column) {
            selection[property] = field;
        }
        // translate an expression
        else if (field instanceof SQL) {
            selection[property] = compiler.expression(field);
        }
        // translate an aliased expression, keeping its alias
        else if (field instanceof SQL.Aliased) {
            const expression = compiler.expression(field.sql);
            selection[property] = Object.assign(expression.as(field.fieldAlias), field, {
                sql: expression,
            });
        }
        // select every column of a table
        else if (field instanceof Table) {
            selection[property] = selectFields(field[TABLE].columns, compiler);
        }
        // translate a nested group
        else {
            selection[property] = selectFields(field, compiler);
        }
    }

    return selection;
}
