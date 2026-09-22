import { schema } from "@destack/schema";
import { ServiceError } from "../error/index.ts";

/** A bounded collection request whose cursor retains its original filters and ordering. */
export const PageRequest = schema.object({
    /** Continuation returned by the preceding page. */
    cursor: schema.string().min(1).optional(),
    /** Maximum records returned in this page. */
    limit: schema.number().int().min(1).max(1000),
});

/** Collection continuation retaining its original scope and sort position. */
export class Page<Position extends schema.Schema> {
    /** Maximum returned records. */
    readonly limit: number;
    /** Exclusive position after the preceding page. */
    readonly after?: schema.Infer<Position>;
    /** Collection and parent identifiers retained in every continuation. */
    readonly scope: readonly string[];

    /** Validate a continuation before constructing its indexed query. */
    constructor(
        input: { cursor?: string; limit?: number },
        scope: readonly string[],
        position: Position,
    ) {
        this.scope = scope;
        this.limit = input.limit ?? 50;
        if (!Number.isInteger(this.limit) || this.limit < 1 || this.limit > 1000) {
            throw new ServiceError("BAD_REQUEST", {
                message: "page limit must be between 1 and 1000",
            });
        }

        // bind cursors to the exact collection and parent identifiers
        if (input.cursor !== undefined) {
            let cursor;
            try {
                cursor = schema
                    .object({ scope: schema.array(schema.string()), after: schema.json() })
                    .parse(JSON.parse(input.cursor));
                this.after = position.parse(cursor.after);
            } catch {
                throw new ServiceError("BAD_REQUEST", { message: "invalid collection cursor" });
            }

            // reject a valid cursor when its collection or filters differ
            if (JSON.stringify(cursor.scope) !== JSON.stringify(scope)) {
                throw new ServiceError("BAD_REQUEST", {
                    message: "cursor belongs to another collection",
                });
            }
        }
    }

    /** Return bounded records and a continuation only when another record exists. */
    result<Item>(rows: Item[], position: (item: Item) => schema.Infer<Position>) {
        const items = rows.slice(0, this.limit);
        const last = items.at(-1);
        const cursor =
            rows.length > this.limit && last !== undefined
                ? JSON.stringify({ scope: this.scope, after: position(last) })
                : null;

        return { items, cursor };
    }
}

/** Describe a page of records and its optional continuation. */
export function page<Item extends schema.Schema>(item: Item) {
    return schema.object({
        /** Records visible to this caller. */
        items: schema.array(item),
        /** Continuation, or null when the collection ends. */
        cursor: schema.string().min(1).nullable(),
    });
}
