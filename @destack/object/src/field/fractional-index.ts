import { BASE_62_DIGITS, generateKeyBetween } from "fractional-indexing";
import { defineSchema, schema } from "@destack/schema";

/** The random digits a placed index ends in, so that two placements in one gap differ. */
const JITTER_DIGITS = 4;

/** A key ordering siblings by byte comparison, a new one always fitting between two others: an integer part its first character sizes, then fraction digits without a trailing zero. */
export const FractionalIndex = Object.assign(
    defineSchema(schema.string().regex(/^[A-Za-z][0-9A-Za-z]*$/u)),
    {
        /** Generate the shortest index between two others, either end open when absent, refusing a malformed index or a pair out of order. */
        between(before: string | undefined, after: string | undefined): string {
            return generateKeyBetween(before ?? null, after ?? null);
        },

        /** Place an index between two others, ending in random digits so that concurrent placements in one gap never collide. */
        place(before: string | undefined, after: string | undefined): string {
            // extend the shortest index between them with random digits, the last never zero, staying below the upper end
            const middle = FractionalIndex.between(before, after);
            const random = [...crypto.getRandomValues(new Uint8Array(JITTER_DIGITS))].map(
                (value, index) =>
                    BASE_62_DIGITS.charAt(
                        index === JITTER_DIGITS - 1
                            ? 1 + (value % (BASE_62_DIGITS.length - 1))
                            : value % BASE_62_DIGITS.length,
                    ),
            );
            const placed = middle + random.join("");

            return after === undefined || placed < after ? placed : middle;
        },
    },
);
/** A key ordering siblings. */
export type FractionalIndex = schema.Infer<typeof FractionalIndex>;
