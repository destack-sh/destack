import { schema } from "@destack/schema";
import { ObjectError } from "../error/error.ts";

import type {} from "@destack/package/import-meta";

/** The rows a page returns unless a list asks for another number. */
export const DEFAULT_PAGE_LIMIT = 50;

/** The most rows one page returns. */
export const MAX_PAGE_LIMIT = 1000;

/** A page position within a collection scope. */
export class Page<Position extends schema.Schema> {
    /** The most records the page returns. */
    readonly limit: number;
    /** The exclusive position after the preceding page. */
    readonly after?: schema.Infer<Position>;
    /** The collection and parent identifiers every cursor has. */
    readonly scope: readonly string[];

    /** Create the page from a request. */
    constructor(
        input: { cursor?: string; limit?: number },
        scope: readonly string[],
        position: Position,
    ) {
        // bind the scope and validate the limit
        this.scope = scope;
        this.limit = input.limit ?? DEFAULT_PAGE_LIMIT;
        if (!Number.isInteger(this.limit) || this.limit < 1 || this.limit > MAX_PAGE_LIMIT) {
            throw ObjectError.of(
                "INVALID_PAGE_LIMIT",
                `page limit must be between 1 and ${MAX_PAGE_LIMIT}`,
            );
        }

        // read the cursor
        if (input.cursor !== undefined) {
            let cursor;
            try {
                cursor = schema
                    .object({ scope: schema.array(schema.string()), after: schema.json() })
                    .parse(JSON.parse(input.cursor));
                this.after = position.parse(cursor.after);
            } catch {
                throw ObjectError.of("INVALID_CURSOR", "invalid collection cursor");
            }

            // reject a cursor of another scope
            if (JSON.stringify(cursor.scope) !== JSON.stringify(scope)) {
                throw ObjectError.of("INVALID_CURSOR", "cursor belongs to another collection");
            }
        }
    }

    /** Return the page's records and a cursor when more exist. */
    result<Item>(rows: readonly Item[], position: (item: Item) => schema.Infer<Position>) {
        // cut to the limit and build the cursor
        const items = rows.slice(0, this.limit);
        const last = items.at(-1);
        const cursor =
            rows.length > this.limit && last !== undefined
                ? JSON.stringify({ scope: this.scope, after: position(last) })
                : null;

        return { items, cursor };
    }
}
