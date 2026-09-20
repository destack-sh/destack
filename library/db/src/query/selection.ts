import { Column as DrizzleColumn, SQL } from "drizzle-orm";
import { Column } from "../table/column.ts";
import { type Select, TABLE, Table } from "../table/table.ts";
import type { SchemaCompiler } from "../dialect/compiler.ts";

/** The source qualifier retained by inferred subquery fields. */
export declare const SOURCE: unique symbol;

/** Fields selected from tables, expressions, or nested groups. */
export interface Selection {
    /** A selected field or nested group. */
    readonly [property: string]: Column | DrizzleColumn | SQL | SQL.Aliased | Table | Selection;
}

/** The application record produced by a selection. */
export type SelectionResult<Fields extends Selection, NullableTables extends string = never> = {
    [Property in keyof Fields]: Fields[Property] extends
        Column<infer Value, infer Required, boolean, infer Name>
        ? Name extends NullableTables ? Value | null : Required extends true ? Value : Value | null
        : Fields[Property] extends DrizzleColumn ? Fields[Property]["_"]["data"]
        : Fields[Property] extends SQL<infer Value> | SQL.Aliased<infer Value>
            ? Fields[Property] extends { readonly [SOURCE]: infer Name }
                ? Name extends NullableTables ? Value | null : Value
            : Value
        : Fields[Property] extends Table
            ? Fields[Property][typeof TABLE]["name"] extends NullableTables
                ? Select<Fields[Property]> | null
            : Select<Fields[Property]>
        : Fields[Property] extends Selection ?
                | SelectionResult<Fields[Property], NullableTables>
                | NullableSelection<Fields[Property], NullableTables>
        : never;
};

/** Tables referenced directly by a nested selection. */
type SelectionTables<Fields extends Selection> = {
    [Property in keyof Fields]: Fields[Property] extends
        Column<unknown, boolean, boolean, infer Name> ? Name
        : Fields[Property] extends { readonly [SOURCE]: infer Name } ? Name
        : never;
}[keyof Fields];

/** A nested selection becomes NULL only when it belongs to one nullable joined table. */
type NullableSelection<
    Fields extends Selection,
    NullableTables extends string,
    Names = SelectionTables<Fields>,
    Name = Names,
> = Name extends NullableTables ? [Names] extends [Name] ? null : never : never;

/** A selection translated to Drizzle fields. */
export interface NativeSelection {
    /** A physical column, SQL expression, or nested group. */
    [property: string]: DrizzleColumn | SQL | SQL.Aliased | NativeSelection;
}

/** Translate selected declarations while retaining property names and nesting. */
export function selectFields(fields: Selection, schema: SchemaCompiler): NativeSelection {
    const selection: NativeSelection = {};

    // preserve SQL objects so their result decoders and aliases remain intact
    for (const [property, field] of Object.entries(fields)) {
        if (field instanceof Column) selection[property] = schema.column(field);
        else if (field instanceof DrizzleColumn) selection[property] = field;
        else if (field instanceof SQL) selection[property] = schema.expression(field);
        else if (field instanceof SQL.Aliased) {
            const expression = schema.expression(field.sql);
            selection[property] = Object.assign(expression.as(field.fieldAlias), field, {
                sql: expression,
            });
        } else if (field instanceof Table) {
            selection[property] = selectFields(field[TABLE].columns, schema);
        } else selection[property] = selectFields(field, schema);
    }

    return selection;
}
