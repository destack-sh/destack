import { expect, test } from "@destack/test";
import { canonicalize } from "./json.ts";

test("serialize JSON with sorted keys and without undefined fields", () => {
    // sort keys at every depth and leave out undefined fields
    const value = { beta: [1, { delta: true, gamma: null }], alpha: "x", skipped: undefined };
    expect(canonicalize(value)).toBe('{"alpha":"x","beta":[1,{"delta":true,"gamma":null}]}');
});

test("refuse values JSON cannot keep", () => {
    // refuse non-finite numbers, class instances and functions
    expect(() => canonicalize(Number.NaN)).toThrow(new TypeError("value is not JSON: number"));
    expect(() => canonicalize({ at: new Date(0) })).toThrow(
        new TypeError("value is not JSON: Date"),
    );
    expect(() => canonicalize([() => 1])).toThrow(new TypeError("value is not JSON: function"));
});
