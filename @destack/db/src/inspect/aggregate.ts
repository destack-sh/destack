import { defineSchema, schema } from "@destack/schema";

/** A column holding an aggregate of the rows referencing each row. */
export const AggregateDescription = defineSchema(
    schema.object({
        /** The SQL name of the table holding the aggregate. */
        table: schema.string().min(1),
        /** The SQL column holding the aggregate. */
        column: schema.string().min(1),
        /** The SQL column identifying the holding table's rows. */
        id: schema.string().min(1),
        /** The SQL name of the aggregated table. */
        source: schema.string().min(1),
        /** The aggregated table's SQL column referencing the holding rows. */
        key: schema.string().min(1),
        /** The aggregate function. */
        function: schema.enum(["count", "sum", "min", "max"]),
        /** The aggregated column, for sums and extremes. */
        value: schema.string().min(1).optional(),
        /** The values the aggregated rows hold. */
        where: schema.array(
            schema.object({
                /** The SQL column. */
                column: schema.string().min(1),
                /** The value, null for none. */
                value: schema.union([
                    schema.string(),
                    schema.number(),
                    schema.boolean(),
                    schema.null(),
                ]),
            }),
        ),
    }),
);
/** A column holding an aggregate of the rows referencing each row. */
export type AggregateDescription = schema.Infer<typeof AggregateDescription>;
