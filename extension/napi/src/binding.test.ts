import { describe, expect, test } from "bun:test";
import { fibonacci } from "./binding";

describe("fibonacci binding", () => {
    test("computes the base sequence", () => {
        const inputValues = [1, 2, 3, 4, 5, 6];
        const computed = inputValues.map((value) => fibonacci(value));
        expect(computed).toEqual([1, 1, 2, 3, 5, 8]);
    });

    test("handles larger inputs", () => {
        expect(fibonacci(10)).toBe(55);
        expect(fibonacci(12)).toBe(144);
    });
});
