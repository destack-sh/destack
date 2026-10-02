import { schema } from "@destack/schema";
import { and, isNull, sql, type SQL } from "../sql/index.ts";
import { TABLE, Table, type Select } from "../table/table.ts";
import type { ColumnValue } from "../table/column.ts";
import { DatabaseError } from "../error/error.ts";
import { Condition, Scalar } from "./condition.ts";

/** A key's name: the table's SQL name, then each key value in JSON form. */
const KeyName = schema.tuple([schema.string()], schema.json());

/** The properties of a table's primary key. */
type KeyProperty<Definition extends Table> = {
    [
        Property in keyof Definition[typeof TABLE]["columns"]
    ]: Definition[typeof TABLE]["columns"][Property]["_"]["key"] extends false ? never : Property;
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
            throw new DatabaseError("INVALID_QUERY", `${table[TABLE].name} has no key`);
        }

        return and(first, ...rest);
    },

    /** Match the rows with any of some keys. */
    any(table: Table, keys: readonly Key[]): Condition {
        return Condition.any(
            ...keys.map((key) =>
                Condition.all(
                    ...table[TABLE].key.map((property) =>
                        Condition.eq(
                            property,
                            Scalar.parse(
                                Key.json(table, property, Key.value(table, key, property)),
                            ),
                        ),
                    ),
                ),
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

    /** Read a row's key from its name, refusing another table's. */
    parse(table: Table, name: string): Key {
        // require a name of this table
        const [owner, ...values] = KeyName.parse(JSON.parse(name));
        if (owner !== table[TABLE].sqlName) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `key name ${name} is not of ${table[TABLE].name}`,
            );
        }

        // read each key value

        return Object.fromEntries(
            table[TABLE].key.map((property, index) => {
                const value = values[index];

                if (value === undefined) {
                    throw new DatabaseError("INVALID_QUERY", `key name ${name} lacks ${property}`);
                }

                return [property, table[TABLE].column(property).definition.fromJson(value)];
            }),
        );
    },

    /** Read one key value of a row, refusing a row without it. */
    value(table: Table, row: Key, property: string): ColumnValue {
        const value = row[property];
        if (value === undefined) {
            throw new DatabaseError(
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
};
