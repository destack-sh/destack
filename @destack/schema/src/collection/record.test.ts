import { expect, test } from "@destack/test";
import { mapValues } from "./record.ts";

test("map each own value by its key into a record of the same keys", () => {
    // map a record's values and keys, leaving its prototype's getter out
    const source = new Pair();
    const mapped = mapValues<Pair, { alpha: string; beta: string }>(
        source,
        (value, key) => `${key}=${value}`,
    );

    expect(mapped).toEqual({ alpha: "alpha=2", beta: "beta=3" });
});

/** Two own values beside a getter its prototype has. */
class Pair {
    /** The first value. */
    readonly alpha = 2;
    /** The second value. */
    readonly beta = 3;

    /** A value the prototype computes. */
    get inherited(): number {
        return 1;
    }
}
