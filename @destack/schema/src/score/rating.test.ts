import { expect, test } from "@destack/test";
import { Rating } from "./rating.ts";

test("take a whole rating from zero, up to the most it is narrowed to", () => {
    const outOfFive = Rating.max(5);
    const taken = [-1, 0, 5, 6, 2.5].map((value) => [
        Rating.safeParse(value).success,
        outOfFive.safeParse(value).success,
    ]);

    expect(taken).toEqual([
        [false, false],
        [true, true],
        [true, true],
        [true, false],
        [false, false],
    ]);
});
