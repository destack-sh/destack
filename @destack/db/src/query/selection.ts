import { Aliased, SQL, sql } from "../sql/index.ts";
import { Column, type ColumnDefinition, type ValueOf } from "../table/column.ts";
import { type Select, TABLE, Table } from "../table/table.ts";
import type { Dialect } from "../dialect/dialect.ts";

/** The fields of a selection: columns, fragments, whole tables or nested groups. */
export interface Selection {
    /** A selected field or nested group. */
    readonly [property: string]: Column | SQL | Aliased | Table | Selection;
}

/** The record a selection produces, with the groups of nullable joined tables nullable. */
export type SelectionResult<Fields extends Selection, NullableTables extends string = never> = {
    [Property in keyof Fields]: Fields[Property] extends Column<infer Definition, infer Name>
        ? Name extends NullableTables
            ? ValueOf<Definition> | null
            : Definition["nullable"] extends false
              ? ValueOf<Definition>
              : ValueOf<Definition> | null
        : Fields[Property] extends SQL<infer Value> | Aliased<infer Value>
          ? Value
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

/** The tables a nested selection reads. */
type SelectionTables<Fields extends Selection> = {
    [Property in keyof Fields]: Fields[Property] extends Column<ColumnDefinition, infer Name>
        ? Name
        : never;
}[keyof Fields];

/** A nested selection of one nullable joined table. */
type NullableSelection<
    Fields extends Selection,
    NullableTables extends string,
    Names = SelectionTables<Fields>,
    Name = Names,
> = Name extends NullableTables ? ([Names] extends [Name] ? null : never) : never;

/** A selected value at its position in the driver row. */
interface Leaf {
    /** The selected column or fragment. */
    readonly field: Column | SQL | Aliased;
    /** The position in the driver row. */
    readonly index: number;
}

/** A selected group: its entries by property, leaves or nested groups. */
interface Group {
    /** The entries in property order. */
    readonly entries: readonly (readonly [string, Leaf | Group])[];
}

/** A selection flattened to fields in order and decoded back into nested records. */
export class Projection {
    /** The selected values in field order. */
    readonly fields: readonly (Column | SQL | Aliased)[];
    /** The record shape over the field positions. */
    readonly #root: Group;

    /** Flatten a selection in property order. */
    constructor(selection: Selection) {
        const fields: (Column | SQL | Aliased)[] = [];
        this.#root = group(selection, fields);
        this.fields = fields;
    }

    /** Render the selected fields, separated by commas. */
    sql(): SQL {
        return sql.join(this.fields, sql.raw(", "));
    }

    /** Decode driver rows into records, a nullable group of only missing values as null. */
    decode<Result>(
        rows: readonly (readonly unknown[])[],
        dialect: Dialect,
        nullable: ReadonlySet<string>,
    ): Result[];
    /** Decode driver rows, whose signature above types them by the selection they decode. */
    decode(
        rows: readonly (readonly unknown[])[],
        dialect: Dialect,
        nullable: ReadonlySet<string>,
    ): unknown[] {
        return rows.map((values) => decodeGroup(this.#root, values, dialect, nullable));
    }
}

/** Collect a selection's fields in property order into a group shape. */
function group(selection: Selection, fields: (Column | SQL | Aliased)[]): Group {
    const entries = Object.entries(selection).map(([property, field]): [string, Leaf | Group] => {
        // keep a selected value at the next position
        if (field instanceof Column || field instanceof SQL || field instanceof Aliased) {
            fields.push(field);

            return [property, { field, index: fields.length - 1 }];
        }

        // nest a table's columns or a nested selection
        return [property, group(field instanceof Table ? field[TABLE].columns : field, fields)];
    });

    return { entries };
}

/** Decode a group's values, or null when every value is missing and every column is nullable. */
function decodeGroup(
    shape: Group,
    values: readonly unknown[],
    dialect: Dialect,
    nullable: ReadonlySet<string>,
): Record<string, unknown> {
    return Object.fromEntries(
        shape.entries.map(([property, entry]) => [
            property,
            "field" in entry
                ? decodeValue(entry.field, values[entry.index], dialect)
                : isMissingGroup(entry, values, nullable)
                  ? null
                  : decodeGroup(entry, values, dialect, nullable),
        ]),
    );
}

/** Report whether a nested group reads only nullable joined columns, all of them missing. */
function isMissingGroup(
    shape: Group,
    values: readonly unknown[],
    nullable: ReadonlySet<string>,
): boolean {
    return (
        shape.entries.length > 0 &&
        shape.entries.every(([, entry]) =>
            "field" in entry
                ? entry.field instanceof Column &&
                  nullable.has(entry.field.table) &&
                  values[entry.index] === null
                : isMissingGroup(entry, values, nullable),
        )
    );
}

/** Decode one selected driver value by its column or fragment decoder, keeping null. */
function decodeValue(field: Column | SQL | Aliased, value: unknown, dialect: Dialect): unknown {
    const decoder =
        field instanceof Column
            ? field
            : field instanceof Aliased
              ? field.sql.decoder
              : field.decoder;
    if (value === undefined) {
        throw new TypeError("the database driver returned fewer values than the query selects");
    } else if (value === null) {
        return null;
    } else if (decoder === undefined) {
        return value;
    }

    return decoder instanceof Column ? decoder.definition.decode(value, dialect) : decoder(value);
}
