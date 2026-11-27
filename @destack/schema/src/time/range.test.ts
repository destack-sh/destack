import { expect, test } from "@destack/test";
import { DateRange } from "./range.ts";

test("make a range of two dates in either order, the earlier first", () => {
    expect([
        DateRange.of("2026-12-24", "2026-12-01"),
        DateRange.of("2026-12-01", "2026-12-24"),
    ]).toEqual([
        { from: "2026-12-01", to: "2026-12-24" },
        { from: "2026-12-01", to: "2026-12-24" },
    ]);
});
