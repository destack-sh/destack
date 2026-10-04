import { expect, test } from "@destack/test";
import { PlainDate } from "./plain.ts";

test("order dates by year, then month, then day", () => {
    const dates: PlainDate[] = [
        { year: 2026, month: 10, day: 4 },
        { year: 2025, month: 12, day: 31 },
        { year: 2026, month: 2, day: 28 },
        { year: 2026, month: 10, day: 1 },
    ];
    expect(dates.toSorted((left, right) => PlainDate.compare(left, right))).toEqual([
        { year: 2025, month: 12, day: 31 },
        { year: 2026, month: 2, day: 28 },
        { year: 2026, month: 10, day: 1 },
        { year: 2026, month: 10, day: 4 },
    ]);
});

test("equal the same day, and nothing that is absent", () => {
    const day = { year: 2026, month: 10, day: 4 };
    expect([
        PlainDate.equals(day, { year: 2026, month: 10, day: 4 }),
        PlainDate.equals(day, { year: 2026, month: 10, day: 5 }),
        PlainDate.equals(day, undefined),
        PlainDate.equals(undefined, undefined),
    ]).toEqual([true, false, false, false]);
});
