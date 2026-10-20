import { LogPosition } from "@destack/db";
import { schema } from "@destack/schema";
import { Watermark } from "@destack/sync";

/** The header with watermarks on requests and responses. */
export const BOOKMARK_HEADER = "Destack-Bookmark";

/** The latest watermark per scope. */
export class Bookmark {
    /** The latest watermark per scope. */
    #watermarks: Watermark[] = [];

    /** Parse a header value. */
    static parse(header: string | null): Bookmark {
        const bookmark = new Bookmark();
        if (header !== null && header !== "") {
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

    /** Merge a watermark, taking the newest per scope. */
    observe(watermark: Watermark): void {
        const index = this.#watermarks.findIndex((known) => known.scope === watermark.scope);
        const known = this.#watermarks[index];

        // add the first watermark of a scope
        if (known === undefined) {
            this.#watermarks.push(watermark);
        }
        // replace a watermark of the log the scope left, or an older one of its log
        else if (watermark.log !== known.log || LogPosition.isAfter(watermark, known)) {
            this.#watermarks[index] = watermark;
        }
    }

    /** Write the watermarks as a header value. */
    format(): string {
        return JSON.stringify(this.#watermarks);
    }
}
