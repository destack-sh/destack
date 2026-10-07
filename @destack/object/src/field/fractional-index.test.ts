import { expect, test } from "@destack/test";
import { FractionalIndex } from "./fractional-index.ts";

test("generate indexes between others, either end open, keeping appended keys short and refusing a malformed index or a pair out of order", () => {
    // append a thousand keys after each other and prepend a thousand before each other
    let last: string | undefined;
    let first: string | undefined;
    for (let step = 0; step < 1000; step++) {
        last = FractionalIndex.between(last, undefined);
        first = FractionalIndex.between(undefined, first);
    }

    expect({
        open: FractionalIndex.between(undefined, undefined),
        between: FractionalIndex.between("a0", "a1"),
        lengths: [last?.length, first?.length],
        refused: [
            () => FractionalIndex.between("a1", "a0"),
            () => FractionalIndex.between("a0", "a0"),
            () => FractionalIndex.between("a00", undefined),
        ].map((generate) => {
            try {
                generate();

                return "generated";
            } catch {
                return "refused";
            }
        }),
        valid: ["a0", "a0V", "Zz", "0a", ""].map((key) => FractionalIndex.safeParse(key).success),
    }).toEqual({
        open: "a0",
        between: "a0V",
        lengths: [3, 3],
        refused: ["refused", "refused", "refused"],
        valid: [true, true, true, false, false],
    });
});

test("place concurrent indexes in one gap apart from each other and between their neighbours, each valid", () => {
    // place many indexes in the same gaps, including one whose middle is a prefix of the upper end
    const gaps: readonly [string | undefined, string | undefined][] = [
        ["a0", "a1"],
        ["a0", "a0V"],
        [undefined, "a0"],
        ["a0", undefined],
    ];
    const placed = gaps.map(([before, after]) =>
        Array.from({ length: 50 }, () => FractionalIndex.place(before, after)),
    );

    expect(
        placed.map((indexes, gap) => {
            const [before, after] = gaps[gap] ?? [];

            return (
                new Set(indexes).size === indexes.length &&
                indexes.every(
                    (index) =>
                        FractionalIndex.safeParse(index).success &&
                        (before === undefined || index > before) &&
                        (after === undefined || index < after),
                )
            );
        }),
    ).toEqual([true, true, true, true]);
});
