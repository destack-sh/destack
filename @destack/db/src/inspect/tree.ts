import { defineSchema, schema } from "@destack/schema";

/** A scoped parent relationship and its engine-maintained ancestor index. */
export const TreeDescription = defineSchema(
    schema.object({
        /** The tree's declaration name. */
        name: schema.string().min(1),
        /** The SQL name of the table holding the nodes. */
        table: schema.string().min(1),
        /** The SQL column holding the node identity. */
        id: schema.string().min(1),
        /** The SQL column holding the tree scope. */
        scope: schema.string().min(1),
        /** The SQL column holding the nullable parent identity. */
        parent: schema.string().min(1),
        /** The SQL name of the ancestor index table. */
        ancestors: schema.string().min(1),
        /** The SQL name of the table serializing hierarchy writes per scope. */
        revision: schema.string().min(1),
    }),
);
/** A scoped parent relationship and its engine-maintained ancestor index. */
export type TreeDescription = schema.Infer<typeof TreeDescription>;
