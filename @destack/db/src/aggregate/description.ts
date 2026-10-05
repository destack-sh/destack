import { defineSchema, schema } from "@destack/schema";
import { Scalar } from "../query/condition.ts";

/** Where an aggregate is kept and which rows it reads. */
const AGGREGATE_TARGET = {
    /** The SQL name of the table storing the aggregate. */
    table: schema.string().min(1),
    /** The SQL column storing the aggregate. */
    column: schema.string().min(1),
    /** The SQL column identifying the target table's rows. */
    id: schema.string().min(1),
    /** The SQL name of the aggregated table. */
    source: schema.string().min(1),
    /** The aggregated table's SQL column referencing the target rows. */
    key: schema.string().min(1),
    /** The values of the aggregated rows. */
    where: schema.array(
        schema.object({
            /** The SQL column. */
            column: schema.string().min(1),
            /** The value, null for none. */
            value: Scalar,
        }),
    ),
};

/** A column with an aggregate of the rows referencing each row: a count, or a sum or extreme of a column. */
export const AggregateDescription = defineSchema(
    schema.discriminatedUnion("function", [
        schema.object({
            ...AGGREGATE_TARGET,
            /** Count the rows. */
            function: schema.literal("count"),
        }),
        schema.object({
            ...AGGREGATE_TARGET,
            /** Sum the values, or take their least or greatest. */
            function: schema.enum(["sum", "min", "max"]),
            /** The aggregated column. */
            value: schema.string().min(1),
        }),
    ]),
);
/** A column with an aggregate of the rows referencing each row. */
export type AggregateDescription = schema.Infer<typeof AggregateDescription>;
