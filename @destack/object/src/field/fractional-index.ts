import type { Random } from "@destack/environment";
import { generateKeyBetween } from "fractional-indexing";
import { schema } from "@destack/schema";

/** The random bytes of a placed index: at 24 bits, two placements in one gap tie once in 16M. */
const JITTER_BYTES = 3;

/** A key ordering siblings by bytes, with room for a new key between any two. */
export const FractionalIndex = Object.assign(schema.string().regex(/^[A-Za-z][0-9A-Za-z]*$/u), {
    /** Generate the shortest index between two others, with open ends when absent. */
    between(before: string | undefined, after: string | undefined): string {
        return generateKeyBetween(before ?? null, after ?? null);
    },

    /** Place an index between two others at random, by halving the gap once per random bit. */
    place(before: string | undefined, after: string | undefined, random: Random): string {
        // go on in the lower or the upper half of the gap at each bit
        let [lower, upper] = [before, after];
        let placed = FractionalIndex.between(lower, upper);
        for (const byte of random.bytes(JITTER_BYTES)) {
            for (let bit = 0; bit < 8; bit++) {
                if (((byte >> bit) & 1) === 0) {
                    upper = placed;
                } else {
                    lower = placed;
                }
                placed = FractionalIndex.between(lower, upper);
            }
        }

        return placed;
    },
});
/** A key ordering siblings. */
export type FractionalIndex = schema.Infer<typeof FractionalIndex>;
