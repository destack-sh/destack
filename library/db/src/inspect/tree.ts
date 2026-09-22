import { defineSchema, schema } from "@destack/schema";

/** A scoped parent relationship and its engine-maintained ancestor index. */
export const TreeDescription = defineSchema(
    schema.object({
        name: schema.string().min(1),
        table: schema.string().min(1),
        id: schema.string().min(1),
        scope: schema.string().min(1),
        parent: schema.string().min(1),
        ancestors: schema.string().min(1),
        revision: schema.string().min(1),
    }),
);
/** A scoped parent relationship and its engine-maintained ancestor index. */
export type TreeDescription = schema.Infer<typeof TreeDescription>;
