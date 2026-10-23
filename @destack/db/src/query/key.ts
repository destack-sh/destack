import { schema } from "@destack/schema";
import { and, isNull, sql, type SQL } from "../sql/index.ts";
import { TABLE, Table, type Select } from "../table/table.ts";
import type { ColumnValue } from "../table/column.ts";
import type { Row } from "../table/row.ts";
import { DatabaseError } from "../error/error.ts";
import { Condition, Scalar } from "./condition.ts";

/** A key's name: the table's SQL name, then each key value in JSON form. */
const KeyName = schema.tuple([schema.string()], schema.json());

/** The properties of a table's primary key, any property of a table whose columns are unknown. */
type KeyProperty<Definition extends Table> =
    string extends keyof Definition[typeof TABLE]["columns"]
        ? string
        : KnownKeyProperty<Definition>;

/** The properties of a known table's primary key. */
type KnownKeyProperty<Definition extends Table> = {
    [
        Property in keyof Definition[typeof TABLE]["columns"]
    ]: Definition[typeof TABLE]["columns"][Property]["definition"] extends {
        readonly primaryKey: true;
    }
        ? Property
        : never;
}[keyof Definition[typeof TABLE]["columns"]];

/** A row's primary key values. */
export type Key<Definition extends Table = Table> = Pick<
    Select<Definition>,
    KeyProperty<Definition>
>;

/** A table's primary key. */
export const Key = {
    /** Match the row with a row's key. */
    match(table: Table, row: Key): SQL<boolean> {
        const [first, ...rest] = table[TABLE].key.map((property) => {
            const column = table[TABLE].column(property);
            const value = Key.value(table, row, property);

            return value === null
                ? isNull(column)
                : sql<boolean>`${column} = ${sql.param(value, column)}`;
        });
        if (first === undefined) {
            throw DatabaseError.of("INVALID_QUERY", `${table[TABLE].name} has no key`);
        }

        return and(first, ...rest);
    },

    /** Match the rows with any of some keys. */
    any(table: Table, keys: readonly Key[]): Condition {
        return { OR: keys.map((key) => Key.where(table, key)) };
    },

    /** Match the row with a key, its values in their JSON form. */
    where(table: Table, key: Key): Condition {
        return Condition.equal(
            Object.fromEntries(
                table[TABLE].key.map((property) => [
                    property,
                    Scalar.parse(Key.json(table, property, Key.value(table, key, property))),
                ]),
            ),
        );
    },

    /** Build a row's key name from its table and key, alike in every dialect. */
    name(table: Table, row: Key): string {
        // write the JSON array of the table name and key values
        const definition = table[TABLE];
        let name = `[${JSON.stringify(definition.sqlName)}`;
        for (const property of definition.key) {
            name += `,${JSON.stringify(Key.json(table, property, Key.value(table, row, property)))}`;
        }

        return `${name}]`;
    },

    /** Read a row's key from a record of its columns, failing for missing or non-scalar values. */
    of(table: Table, row: Row): Key {
        const key: Record<string, ColumnValue> = {};
        for (const property of table[TABLE].key) {
            const value = row[property];
            if (
                typeof value !== "string" &&
                typeof value !== "number" &&
                typeof value !== "bigint" &&
                typeof value !== "boolean" &&
                !(value instanceof Uint8Array)
            ) {
                throw DatabaseError.of(
                    "INVALID_QUERY",
                    `${table[TABLE].name} row has no key value ${property}`,
                );
            }
            key[property] = value;
        }

        return key;
    },

    /** Read the key a condition selects by equal values of exactly the key's columns, if any. */
    from(table: Table, where: Condition): Key | undefined {
        // take only a plain equality on each key column and nothing else
        const properties = table[TABLE].key;
        const key: Record<string, ColumnValue> = {};
        for (const [property, value] of Object.entries(where)) {
            if (
                !properties.includes(property) ||
                (typeof value !== "string" &&
                    typeof value !== "number" &&
                    typeof value !== "boolean")
            ) {
                return undefined;
            }
            key[property] = table[TABLE].column(property).definition.fromJson(value);
        }

        return Object.keys(key).length === properties.length ? key : undefined;
    },

    /** Report whether a row is one of its table's predefined rows, by key. */
    isPredefined(table: Table, row: Key): boolean {
        // skip keying a table without predefined rows
        const { predefined } = table[TABLE];
        if (predefined.length === 0) {
            return false;
        }

        // compare its key name with each predefined row's
        const name = Key.name(table, row);

        return predefined.some((entry) => Key.name(table, entry) === name);
    },

    /** Read a row's key from its name, refusing another table's. */
    parse(table: Table, name: string): Key {
        // require a name of this table
        const [owner, ...values] = KeyName.parse(JSON.parse(name));
        if (owner !== table[TABLE].sqlName) {
            throw DatabaseError.of(
                "INVALID_QUERY",
                `key name ${name} does not belong to ${table[TABLE].name}`,
            );
        }

        // read each key value

        return Object.fromEntries(
            table[TABLE].key.map((property, index) => {
                const value = values[index];

                if (value === undefined) {
                    throw DatabaseError.of(
                        "INVALID_QUERY",
                        `key name ${name} is missing ${property}`,
                    );
                }

                return [property, table[TABLE].column(property).definition.fromJson(value)];
            }),
        );
    },

    /** Read one key value of a row, refusing a row without it. */
    value(table: Table, row: Key, property: string): ColumnValue {
        const value = row[property];
        if (value === undefined) {
            throw DatabaseError.of(
                "INVALID_QUERY",
                `${table[TABLE].name} row has no key value ${property}`,
            );
        }

        return value;
    },

    /** Write one key value in its JSON form. */
    json(table: Table, property: string, value: ColumnValue): ColumnValue {
        return value === null ? null : table[TABLE].column(property).definition.toJson(value);
    },

    /** Write a key's values in JSON form, in key order. */
    tuple(table: Table, key: Key): ColumnValue[] {
        return table[TABLE].key.map((property) =>
            Key.json(table, property, Key.value(table, key, property)),
        );
    },
};
