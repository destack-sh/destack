import { Alias, SQL, sql } from "../sql/index.ts";
import { Column, type ColumnDefinition, type ValueOf } from "../table/column.ts";
import { type Select, TABLE, Table } from "../table/table.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { ColumnCipher, SEALED_DIALECT } from "../database/cipher.ts";

/** The fields of a selection: columns, fragments, whole tables or nested groups. */
export interface Selection {
    /** A selected field or nested group. */
    readonly [property: string]: Column | SQL | Alias | Table | Selection;
}

/** The record a selection produces, with the groups of nullable joined tables nullable. */
export type SelectionResult<Fields extends Selection, NullableTables extends string = never> = {
    [Property in keyof Fields]: Fields[Property] extends Column<infer Definition, infer Name>
        ? Name extends NullableTables
            ? ValueOf<Definition> | null
            : Definition["nullable"] extends false
              ? ValueOf<Definition>
              : ValueOf<Definition> | null
        : Fields[Property] extends SQL<infer Value> | Alias<infer Value>
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
    readonly field: Column | SQL | Alias;
    /** The position in the driver row. */
    readonly index: number;
}

/** A selected group: its entries by property, leaves or nested groups. */
interface Group {
    /** The entries in property order. */
    readonly entries: readonly (readonly [string, Leaf | Group])[];
}

/** A selection flattened to fields in order and decoded back into nested records. */
export class Projection<Selected extends Selection = Selection> {
    /** The selected values in field order. */
    readonly fields: readonly (Column | SQL | Alias)[];
    /** The record shape over the field positions. */
    readonly #root: Group;

    /** Flatten a selection in property order. */
    constructor(selection: Selected) {
        const fields: (Column | SQL | Alias)[] = [];
        this.#root = group(selection, fields);
        this.fields = fields;
    }

    /** Render the selected fields with commas, aliasing named fragments for grouping and order. */
    sql(): SQL {
        return sql.join(
            this.fields.map((field) =>
                field instanceof Alias
                    ? sql`${field.sql} AS ${sql.identifier(field.alias)}`
                    : field,
            ),
            sql.raw(", "),
        );
    }

    /** Decode driver rows into selection records, a nullable group of missing values as null. */
    decode(
        rows: readonly (readonly unknown[])[],
        dialect: Dialect,
        nullable: ReadonlySet<string>,
        cipher: ColumnCipher | undefined,
    ): SelectionResult<Selected>[];
    /**
     * Decode driver rows into records shaped as the selection.
     *
     * @construct each record nests groups and decodes fields by column or decoder, as typed.
     */
    decode(
        rows: readonly (readonly unknown[])[],
        dialect: Dialect,
        nullable: ReadonlySet<string>,
        cipher: ColumnCipher | undefined,
    ): unknown[] {
        return rows.map((values) => decodeGroup(this.#root, values, dialect, nullable, cipher));
    }
}

/** Collect a selection's fields in property order into a group shape. */
function group(selection: Selection, fields: (Column | SQL | Alias)[]): Group {
    const entries = Object.entries(selection).map(([property, field]): [string, Leaf | Group] => {
        // keep a selected value at the next position
        if (field instanceof Column || field instanceof SQL || field instanceof Alias) {
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
    cipher: ColumnCipher | undefined,
): Record<string, unknown> {
    return Object.fromEntries(
        shape.entries.map(([property, entry]) => [
            property,
            decodeEntry(entry, values, dialect, nullable, cipher),
        ]),
    );
}

/** Decode one group entry: a field's value, or a nested group's record, null when missing. */
function decodeEntry(
    entry: Leaf | Group,
    values: readonly unknown[],
    dialect: Dialect,
    nullable: ReadonlySet<string>,
    cipher: ColumnCipher | undefined,
): unknown {
    // decode a field
    if ("field" in entry) {
        return decodeValue(entry.field, values[entry.index], dialect, cipher);
    }
    // read a missing nested group as null
    else if (isMissingGroup(entry, values, nullable)) {
        return null;
    }
    // decode a nested group
    else {
        return decodeGroup(entry, values, dialect, nullable, cipher);
    }
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

/** Decode one selected driver value by its column or fragment decoder, opening a sealed one. */
function decodeValue(
    field: Column | SQL | Alias,
    value: unknown,
    dialect: Dialect,
    cipher: ColumnCipher | undefined,
): unknown {
    // read the column definition decoding the value: a column's, or a fragment's mapped to one
    const decoder = field instanceof Column ? field.definition : field.getSQL().decoder;
    const definition = typeof decoder === "object" ? decoder : undefined;
    if (value === undefined) {
        throw new TypeError("the database driver returned fewer values than the query selects");
    } else if (value === null) {
        return null;
    }
    // open a sealed column's value, also a mutation's returned column
    else if (definition?.sealing !== undefined) {
        return definition.decode(
            ColumnCipher.require(cipher, { definition }).open(value, definition.name),
            SEALED_DIALECT,
        );
    } else if (definition !== undefined) {
        return definition.decode(value, dialect);
    }

    return field.getSQL().decode(value, dialect);
}
