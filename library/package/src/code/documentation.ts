import { defineSchema, schema } from "@destack/schema";

/** Documentation attached to a declaration. */
export const Documentation = defineSchema(schema.object({
    /** The rendered documentation comment. */
    text: schema.string(),
    /** Documentation tags in source order. */
    tags: schema.array(schema.object({
        /** The tag name without its leading at sign. */
        name: schema.string(),
        /** The tag contents. */
        text: schema.string().optional(),
    })),
}));
