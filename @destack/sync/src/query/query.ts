import type { Table } from "@destack/db";
import type { Computed, Condition, Order } from "@destack/db/query";
import { defineSchema, schema } from "@destack/schema";

/**
 * Rows of one logged table a subscriber keeps, with what each row includes, or aggregates of them.
 *
 * The rows are those of some scopes that meet a condition.
 * With an order and a limit, it holds only the first rows of that order.
 */
export interface Query {
    /** The logged table. */
    readonly table: Table;
    /** The scopes whose rows the query holds. */
    readonly scopes: readonly string[];
    /** Values computed from each row's logged columns, by name, which conditions, orders, groups and measures read like columns. */
    readonly compute?: Computed;
    /** The condition the rows meet, over logged columns and computed values. */
    readonly where?: Condition;
    /** How the rows sort, completed by the primary key. */
    readonly order?: Order;
    /** The most rows the query holds, the first of its order. */
    readonly limit?: number;
    /** The rows each held row includes, by name. */
    readonly include?: Readonly<Record<string, Include>>;
    /** The relations the condition's `exists` terms follow, by name. */
    readonly relations?: Readonly<Record<string, Relation>>;
    /** Aggregates of the rows the query selects, held instead of the rows. */
    readonly aggregate?: Aggregate;
}

/** Rows each held row includes: those its path reaches that meet a condition, the first of an order per held row. */
export interface Include extends Omit<Query, "scopes"> {
    /** How the included rows relate to a held row. */
    readonly on: Path;
}

/** Rows related to a held row, which an `exists` term asks about and the subscriber holds as witnesses. */
export interface Relation {
    /** The related table, logged in the same scopes. */
    readonly table: Table;
    /** How the related rows relate to a held row: a key or junction path. */
    readonly on: Path;
    /** The condition every related row meets, beside the one an `exists` term asks about. */
    readonly where?: Condition;
    /** The relations the related rows' conditions follow, by name. */
    readonly relations?: Readonly<Record<string, Relation>>;
}

/**
 * How included rows relate to a held row.
 *
 * A key path joins a column of the included row to a column of the held row.
 * A junction path joins them through the rows of a join table, which the include holds too.
 * A descendants or ancestors path follows a parent column of the held row's own table, down or up, through rows the audience sees.
 */
export type Path =
    | {
          /** Join a column of the included row to one of the held row. */
          readonly kind: "key";
          /** The included row's column, by property. */
          readonly column: string;
          /** The held row's column, by property. */
          readonly parent: string;
      }
    | {
          /** Join through the rows of a join table. */
          readonly kind: "junction";
          /** The join table, logged in the same scopes. */
          readonly table: Table;
          /** The join table's column naming the held row, and the held row's column it holds. */
          readonly from: { readonly column: string; readonly key: string };
          /** The join table's column naming the included row, and the included row's column it holds. */
          readonly to: { readonly column: string; readonly key: string };
      }
    | {
          /** Follow a parent column down to every row below the held row, or up to every row above it. */
          readonly kind: "descendants" | "ancestors";
          /** The column naming a row's parent, by property, holding the key of another row of the table. */
          readonly column: string;
      };

/** One measure of a group: its row count, or a function of one column. */
export const Measure = defineSchema(
    schema.object({
        /** The function. */
        function: schema.enum(["count", "sum", "avg", "min", "max"]),
        /** The measured column, by property, absent for a count of rows. */
        column: schema.string().min(1).optional(),
    }),
);
/** One measure of a group: its row count, or a function of one column. */
export type Measure = schema.Infer<typeof Measure>;

/** Aggregates of a query's rows, per group of equal column values, and per held row for an include. */
export const Aggregate = defineSchema(
    schema.object({
        /** The columns whose values form the groups, by property; one group when absent. */
        groupBy: schema.array(schema.string().min(1)).optional(),
        /** The measures of each group, by name. */
        values: schema.record(schema.string(), Measure),
    }),
);
/** Aggregates of a query's rows, per group of equal column values, and per held row for an include. */
export type Aggregate = schema.Infer<typeof Aggregate>;
