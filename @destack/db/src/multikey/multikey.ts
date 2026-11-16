import { type JsonValue, schema } from "@destack/schema";
import { DatabaseError } from "../error/index.ts";
import {
    Column,
    ElementKey,
    integer,
    json,
    text,
    type ColumnDefinition,
    type ElementKind,
} from "../table/column.ts";
import { index, type Index, type IndexKey, type TableConstraint } from "../table/constraint.ts";
import { defineTable, TABLE, type Table } from "../table/table.ts";
import type { MultikeyIndexDescription } from "./description.ts";

/** An entry field's name: a copied column's property, or an element path in camel case. */
type EntryField<Key> =
    Key extends ElementKey<infer Path>
        ? CamelPath<Path>
        : Key extends Column<ColumnDefinition, string, infer Property>
          ? Property
          : never;

/** A dotted path in camel case, as in `node.type` to `nodeType`. */
type CamelPath<Path extends string> = Path extends `${infer Head}.${infer Tail}`
    ? `${Head}${Capitalize<CamelPath<Tail>>}`
    : Path;

/** How an entry stores a key: an element key's kind, or a copied column's kind. */
type EntryKind<Key> =
    Key extends ElementKey<string, infer Kind>
        ? Kind
        : Key extends Column<infer Definition>
          ? Extract<Definition["kind"], ElementKind>
          : never;

/** The column definition an entry field of a kind has. */
type EntryDefinition<Kind> = Kind extends "integer"
    ? ReturnType<typeof integer>["definition"]
    : Kind extends "json"
      ? ReturnType<typeof json<JsonValue>>["definition"]
      : ReturnType<typeof text>["definition"];

/** The columns of an entry table: the source row, the element's place, and each key. */
type EntryColumns<TableName extends string, Keys extends readonly IndexKey[]> = {
    /** The source row. */
    readonly row: Column<
        ReturnType<ReturnType<typeof text>["primaryKey"]>["definition"],
        TableName,
        "row"
    >;
    /** The element's place in the array. */
    readonly position: Column<
        ReturnType<ReturnType<typeof integer>["primaryKey"]>["definition"],
        TableName,
        "position"
    >;
} & {
    readonly [Key in Keys[number] as EntryField<Key>]: Column<
        EntryDefinition<EntryKind<Key>>,
        TableName,
        EntryField<Key> & string
    >;
};

/** The entry table of a multikey index, typed by the index's keys. */
export type EntryTable<Declared> =
    Declared extends Index<infer Name, infer Keys>
        ? Table<`${Name}_entry`, EntryColumns<`${Name}_entry`, Keys>> &
              EntryColumns<`${Name}_entry`, Keys>
        : Table;

/** The names of the multikey indexes among a table's constraints. */
export type MultikeyName<Constraints extends readonly TableConstraint[]> = string &
    (Constraints[number] extends infer Declared
        ? Declared extends Index<infer Name, infer Keys>
            ? Extract<Keys[number], ElementKey> extends never
                ? never
                : Name
            : never
        : never);

/** A multikey index over a table: an entry table with a row per array element, kept by triggers. */
export class MultikeyIndex {
    /** The multikey index. */
    readonly index: Index;
    /** The table whose rows the entries derive from. */
    readonly source: Table;
    /** The entry table: each entry's source row, its place in the array, and its keys. */
    readonly table: Table;
    /** Each key's entry property: copied columns by property, element keys by their path. */
    readonly #fields: readonly (readonly [string, Column | ElementKey])[];
    /** The JSON array column whose elements get entries. */
    readonly #array: Column;

    /** Derive the entry table of a multikey index over a source table. */
    constructor(source: Table, multikey: Index) {
        // require copied columns of the table and element keys of one array column
        this.index = multikey;
        this.source = source;
        this.#fields = multikey.columns.map((key) => MultikeyIndex.#field(source, multikey, key));
        const arrays = new Set(
            multikey.columns.flatMap((key) => (key instanceof ElementKey ? [key.column] : [])),
        );
        const [array] = arrays;
        if (array === undefined || arrays.size > 1) {
            throw DatabaseError.of(
                "MIGRATION_BROKEN",
                `index '${multikey.name}' is invalid: it keys elements of ${arrays.size} array columns, where one is allowed`,
            );
        }
        this.#array = array;

        // declare the entry table, indexed by the keys in order and then the row
        const name = `${multikey.name}_entry`;
        const columns = {
            row: text("row").primaryKey(),
            position: integer("position").primaryKey(),
            ...Object.fromEntries(
                this.#fields.map(([field, key]) => [
                    field,
                    MultikeyIndex.#column(multikey, field, key),
                ]),
            ),
        };
        this.table = defineTable(
            name,
            columns,
            {
                constraints: (entry: Readonly<Record<string, Column>>) => {
                    const [first, ...rest] = [
                        ...this.#fields.map(([field]) => MultikeyIndex.#entry(entry, field)),
                        MultikeyIndex.#entry(entry, "row"),
                    ];

                    return [index(`${name}_keys`).on(first, ...rest)];
                },
            },
            { package: source[TABLE].package },
        );
    }

    /** Describe the index's physical columns. */
    describe(): MultikeyIndexDescription {
        // find the source table's identity column
        const source = this.source[TABLE];
        const id = Object.values(source.columns).find(
            (column) => column.definition.primaryKey === true,
        );
        if (id === undefined) {
            throw DatabaseError.of(
                "MIGRATION_BROKEN",
                `index '${this.index.name}' needs a table with a primary key`,
            );
        }

        return {
            name: this.index.name,
            table: source.sqlName,
            id: id.definition.name,
            from: this.#array.definition.name,
            isSparse: this.index.isSparse,
            fields: this.#fields.map(([field, key]) =>
                key instanceof ElementKey
                    ? {
                          column: MultikeyIndex.column(field),
                          path: key.path.split("."),
                          kind: key.kind,
                      }
                    : {
                          column: MultikeyIndex.column(field),
                          source: key.definition.name,
                          kind: MultikeyIndex.#kind(this.index, key),
                      },
            ),
            entryTable: this.table[TABLE].sqlName,
        };
    }

    /** Name an entry property's SQL column. */
    static column(field: string): string {
        return field.replaceAll(/[A-Z]/gu, (letter) => `_${letter.toLowerCase()}`);
    }

    /** Name a key's entry property: a copied column's property, or an element key's path. */
    static #field(
        source: Table,
        multikey: Index,
        key: Index["columns"][number],
    ): [string, Column | ElementKey] {
        // name an element key by its path in camel case
        if (key instanceof ElementKey) {
            const field = key.path.replaceAll(/\.(\w)/gu, (_, letter: string) =>
                letter.toUpperCase(),
            );

            return [field, key];
        }

        // name a copied column of the source table by its property
        if (key instanceof Column && key.table === source[TABLE].sqlName) {
            return [key.property, key];
        }
        // refuse a column of another table
        else if (key instanceof Column) {
            throw DatabaseError.of(
                "MIGRATION_BROKEN",
                `index '${multikey.name}' is invalid: it keys column '${key.property}' of another table`,
            );
        }

        // refuse an expression
        throw DatabaseError.of(
            "MIGRATION_BROKEN",
            `index '${multikey.name}' is invalid: it keys an expression, where a multikey index takes columns and elements`,
        );
    }

    /** Declare the entry column of a key. */
    static #column(multikey: Index, field: string, key: Column | ElementKey) {
        const kind = key instanceof ElementKey ? key.kind : MultikeyIndex.#kind(multikey, key);
        const name = MultikeyIndex.column(field);

        return kind === "text"
            ? text(name)
            : kind === "integer"
              ? integer(name)
              : json(name, schema.json());
    }

    /** Read how a copied column is stored, refusing a kind an entry cannot store. */
    static #kind(multikey: Index, column: Column): ElementKind {
        const { kind } = column.definition;
        if (kind !== "text" && kind !== "integer" && kind !== "json") {
            throw DatabaseError.of(
                "MIGRATION_BROKEN",
                `index '${multikey.name}' cannot copy column '${column.definition.name}': entries store text, integers and JSON`,
            );
        }

        return kind;
    }

    /** Read an entry column by property. */
    static #entry(entry: Readonly<Record<string, Column>>, field: string): Column {
        const column = entry[field];
        if (column === undefined) {
            throw new TypeError(`entry table has no column '${field}'`);
        }

        return column;
    }
}

/** List tables with the entry tables of their multikey indexes after them. */
export function expandMultikeyIndexes(tables: readonly Table[]): Table[] {
    return [
        ...new Set(
            tables.flatMap((table) => [
                table,
                ...table[TABLE].multikeyIndexes.map((each) => each.table),
            ]),
        ),
    ];
}
