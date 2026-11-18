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

test("map a `__proto__` key to an own value, leaving the record's prototype alone", () => {
    // map a record with a plain key and an own `__proto__` key
    const source: Record<string, number> = { plain: 2 };
    Object.defineProperty(source, "__proto__", {
        value: 1,
        enumerable: true,
        writable: true,
        configurable: true,
    });
    const mapped = mapValues<Record<string, number>, Record<string, number>>(
        source,
        (value) => value * 10,
    );

    expect({
        own: Object.getOwnPropertyNames(mapped),
        value: mapped["__proto__"],
        prototype: Object.getPrototypeOf(mapped) === Object.prototype,
    }).toEqual({ own: ["plain", "__proto__"], value: 10, prototype: true });
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
