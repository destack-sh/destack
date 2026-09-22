import { schema } from "@destack/schema";

/** A bounded collection request whose cursor retains its original filters and ordering. */
export const PageRequest = schema.object({
    /** Continuation returned by the preceding page. */
    cursor: schema.string().min(1).optional(),
    /** Maximum records returned in this page. */
    limit: schema.number().int().min(1).max(1000),
});

/** Describe a page of records and its optional continuation. */
export function page<Item extends schema.Schema>(item: Item) {
    return schema.object({
        /** Records visible to this caller. */
        items: schema.array(item),
        /** Continuation, or null when the collection ends. */
        cursor: schema.string().min(1).nullable(),
    });
}
