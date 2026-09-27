import { and, eq, isNull, type SQL } from "drizzle-orm";
import { TABLE, type Table } from "../table/table.ts";
import { Condition, type Scalar } from "./condition.ts";

/** A table's primary key: the properties holding it, and matching and naming rows by it. */
export const Key = {
    /** Match the row holding a row's key. */
    match(table: Table, row: Readonly<Record<string, unknown>>): SQL {
        const columns = table[TABLE].columns;

        return and(
            ...table[TABLE].key.map((name) =>
                row[name] === null || row[name] === undefined
                    ? isNull(columns[name]!)
                    : eq(columns[name]!, row[name]),
            ),
        )!;
    },

    /** Match the rows holding any of some keys by their JSON values; callers chunk keys by CHAIN_TERMS. */
    any(table: Table, keys: readonly Readonly<Record<string, unknown>>[]): Condition {
        const columns = table[TABLE].columns;

        return Condition.any(
            ...keys.map((key) =>
                Condition.all(
                    ...table[TABLE].key.map((property) =>
                        Condition.eq(
                            property,
                            columns[property]!.definition.toJson(key[property]) as Scalar,
                        ),
                    ),
                ),
            ),
        );
    },

    /** Name a row by its table and the JSON form of its key, the same in every dialect. */
    name(table: Table, row: Readonly<Record<string, unknown>>): string {
        const columns = table[TABLE].columns;

        return JSON.stringify([
            table[TABLE].sqlName,
            ...table[TABLE].key.map((property) =>
                row[property] === null || row[property] === undefined
                    ? null
                    : columns[property]!.definition.toJson(row[property]),
            ),
        ]);
    },

    /** Read a row's key back from its name. */
    parse(table: Table, name: string): Record<string, unknown> {
        const columns = table[TABLE].columns;
        const [, ...values] = JSON.parse(name) as unknown[];

        return Object.fromEntries(
            table[TABLE].key.map((property, index) => [
                property,
                values[index] === null
                    ? null
                    : columns[property]!.definition.fromJson(values[index]),
            ]),
        );
    },
};
