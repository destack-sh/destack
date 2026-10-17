import { BASE_62_DIGITS, generateKeyBetween } from "fractional-indexing";
import { schema } from "@destack/schema";

/** The random digits a placed index ends in, so that two placements in one gap differ. */
const JITTER_DIGITS = 4;

/** A key ordering siblings by bytes, with room for a new key between any two. */
export const FractionalIndex = Object.assign(schema.string().regex(/^[A-Za-z][0-9A-Za-z]*$/u), {
    /** Generate the shortest index between two others, with open ends when absent. */
    between(before: string | undefined, after: string | undefined): string {
        return generateKeyBetween(before ?? null, after ?? null);
    },

    /** Place an index between two others with random final digits. */
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
});
/** A key ordering siblings. */
export type FractionalIndex = schema.Infer<typeof FractionalIndex>;
