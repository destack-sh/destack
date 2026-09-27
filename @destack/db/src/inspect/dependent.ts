import { defineSchema, schema } from "@destack/schema";

/** Rows of another table referencing each row of a table under a condition, deleted with it or keeping it, as data. */
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
        /** The values the dependent rows' SQL columns hold. */
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
        /** Delete the dependent rows with the referenced row, or refuse deleting it. */
        onDelete: schema.enum(["cascade", "restrict"]),
    }),
);
/** Rows of another table referencing each row of a table under a condition, deleted with it or keeping it, as data. */
export type DependentDescription = schema.Infer<typeof DependentDescription>;
