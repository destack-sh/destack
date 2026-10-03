import { defineSchema, schema } from "@destack/schema";
import type { Condition } from "./condition.ts";
import type { OrderBy } from "./order.ts";
import type { Extras } from "./namespace.ts";

/** One measure of a group: its row count, or a function of one column. */
export const Measure = defineSchema(
    schema.discriminatedUnion("function", [
        schema.object({
            /** Count the rows. */
            function: schema.literal("count"),
        }),
        schema.object({
            /** Add up, average, or take the least or greatest value. */
            function: schema.enum(["sum", "avg", "min", "max"]),
            /** The measured column. */
            column: schema.string().min(1),
        }),
    ]),
);
/** One measure of a group: its row count, or a function of one column. */
export type Measure = schema.Infer<typeof Measure>;

/** Aggregates of a query's rows per group. */
export const Aggregate = defineSchema(
    schema.object({
        /** The group columns, one group when absent. */
        groupBy: schema.array(schema.string().min(1)).exactOptional(),
        /** The measures of each group, by name. */
        values: schema.record(schema.string(), Measure),
    }),
);
/** Aggregates of a query's rows per group. */
export type Aggregate = schema.Infer<typeof Aggregate>;

/** What a query selects of a table's rows and their relations. */
export interface QueryOptions {
    /** The values computed from each row, read like fields, by name. */
    readonly extras?: Extras;
    /** The condition over logged columns and computed values. */
    readonly where?: Condition;
    /** How the rows sort, completed by the primary key. */
    readonly orderBy?: OrderBy;
    /** The most rows selected, per selected row for an included relation. */
    readonly limit?: number;
    /** The related rows each selected row includes, by relation name: every row, or a shaped selection. */
    readonly with?: Readonly<Record<string, true | QueryOptions>>;
    /** The aggregates kept instead of rows. */
    readonly aggregate?: Aggregate;
}
