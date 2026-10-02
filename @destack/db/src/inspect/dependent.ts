import { defineSchema, schema } from "@destack/schema";
import { Scalar } from "../query/condition.ts";

/** Rows referencing each row of a table, cascading or restricting its deletion. */
export const DependentDescription = defineSchema(
    schema.object({
        /** The SQL name of the referenced table. */
        table: schema.string().min(1),
        /** The SQL column identifying the referenced table's rows. */
        id: schema.string().min(1),
        /** The SQL name of the dependent table. */
        source: schema.string().min(1),
        /** The dependent table's SQL column referencing the rows. */
        key: schema.string().min(1),
        /** The values of the dependent rows. */
        where: schema.array(
            schema.object({
                /** The SQL column. */
                column: schema.string().min(1),
                /** The value, null for none. */
                value: Scalar,
            }),
        ),
        /** Cascade or restrict the deletion. */
        onDelete: schema.enum(["cascade", "restrict"]),
    }),
);
/** Rows referencing each row of a table, cascading or restricting its deletion. */
export type DependentDescription = schema.Infer<typeof DependentDescription>;
