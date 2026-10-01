import { LogPosition } from "@destack/db/log";
import { defineSchema, schema } from "@destack/schema";

/** The header carrying watermarks on requests and responses. */
export const BOOKMARK_HEADER = "destack-bookmark";

/** A position in one scope's log. */
export const Watermark = defineSchema(
    schema.object({
        /** The scope whose database holds the log. */
        scope: schema.string().min(1),
        /** The log's epoch, renewed when the database is restored. */
        epoch: schema.string().min(1),
        /** The log sequence reached within the epoch. */
        sequence: schema.number().int().nonnegative(),
    }),
);
/** A position in one scope's log. */
export type Watermark = schema.Infer<typeof Watermark>;

/** The latest watermark per scope. */
export class Bookmark {
    /** The latest watermark per scope. */
    #watermarks: Watermark[] = [];

    /** Parse a header value. */
    static parse(header: string | null): Bookmark {
        const bookmark = new Bookmark();
        if (header) {
            for (const watermark of schema.array(Watermark).parse(JSON.parse(header))) {
                bookmark.observe(watermark);
            }
        }

        return bookmark;
    }

    /** The latest watermark per scope. */
    get watermarks(): readonly Watermark[] {
        return this.#watermarks;
    }

    /** Merge a watermark and keep the newest per scope. */
    observe(watermark: Watermark): void {
        const index = this.#watermarks.findIndex((known) => known.scope === watermark.scope);

        // add the first watermark of a scope
        if (index === -1) {
            this.#watermarks.push(watermark);
        }
        // replace an older watermark of the scope
        else if (LogPosition.isAfter(watermark, this.#watermarks[index])) {
            this.#watermarks[index] = watermark;
        }
    }

    /** Write the watermarks as a header value. */
    format(): string {
        return JSON.stringify(this.#watermarks);
    }
}
