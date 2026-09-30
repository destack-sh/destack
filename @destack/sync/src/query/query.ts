import type { Table } from "@destack/db";
import type { Computed, Condition, Order } from "@destack/db/query";
import { defineSchema, schema } from "@destack/schema";

/** Rows of one logged table a subscriber keeps, with their includes, or aggregates of them. */
export interface Query {
    /** The logged table. */
    readonly table: Table;
    /** The scopes whose rows the query holds, or every scope. */
    readonly scopes: readonly string[] | "every";
    /** The computed values, by name. */
    readonly compute?: Computed;
    /** The condition over logged columns and computed values. */
    readonly where?: Condition;
    /** How the rows sort, completed by the primary key. */
    readonly order?: Order;
    /** The most rows the query holds. */
    readonly limit?: number;
    /** The rows each held row includes, by name. */
    readonly include?: Readonly<Record<string, Include>>;
    /** The relations of the condition's `exists` terms. */
    readonly relations?: Readonly<Record<string, Relation>>;
    /** The aggregates held instead of rows. */
    readonly aggregate?: Aggregate;
}

/**
 * Rows each held row includes.
 *
 * The included rows live in the query's scopes.
 * A key path on their scope column reaches the rows of every scope instead, those in the scope a held row is.
 */
export interface Include extends Omit<Query, "scopes"> {
    /** How the included rows relate to a held row. */
    readonly on: Path;
}

/** Rows related to a held row. */
export interface Relation {
    /** The related table, logged in the same scopes. */
    readonly table: Table;
    /** How the related rows relate to a held row. */
    readonly on: Path;
    /** The condition every related row meets. */
    readonly where?: Condition;
    /** The relations of the related rows' conditions. */
    readonly relations?: Readonly<Record<string, Relation>>;
}

/**
 * How included rows relate to a held row.
 *
 * A key path joins two columns.
 * A junction path joins through a join table.
 * A descendants or ancestors path follows a parent column.
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
          /** The join table's column naming the held row, and the held column. */
          readonly from: { readonly column: string; readonly key: string };
          /** The join table's column naming the included row, and the included column. */
          readonly to: { readonly column: string; readonly key: string };
      }
    | {
          /** Follow a parent column down or up. */
          readonly kind: "descendants" | "ancestors";
          /** The column naming a row's parent. */
          readonly column: string;
      };

/** One measure of a group: its row count, or a function of one column. */
export const Measure = defineSchema(
    schema.object({
        /** The function. */
        function: schema.enum(["count", "sum", "avg", "min", "max"]),
        /** The measured column, absent for a row count. */
        column: schema.string().min(1).optional(),
    }),
);
/** One measure of a group: its row count, or a function of one column. */
export type Measure = schema.Infer<typeof Measure>;

/** Aggregates of a query's rows per group. */
export const Aggregate = defineSchema(
    schema.object({
        /** The group columns, one group when absent. */
        groupBy: schema.array(schema.string().min(1)).optional(),
        /** The measures of each group, by name. */
        values: schema.record(schema.string(), Measure),
    }),
);
/** Aggregates of a query's rows per group. */
export type Aggregate = schema.Infer<typeof Aggregate>;
