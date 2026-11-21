import { PlainDate } from "./plain.ts";
import { Week } from "./week.ts";
import { expect, test } from "@destack/test";
import { MonthGrid } from "./month.ts";

/** The week of the United States: Sunday first. */
const US: Week = { firstDay: 7, weekend: [6, 7], minimalDays: 1 };

/** The week of Germany and Austria: Monday first, numbered as ISO 8601. */
const GERMANY: Week = { firstDay: 1, weekend: [6, 7], minimalDays: 4 };

/** Write a grid's weeks as their dates' days of month. */
function daysOf(grid: MonthGrid): number[][] {
    return grid.weeks.map((week) => week.map((date) => PlainDate.day(date)));
}

test("lay November 2026 out from Sunday in the US and from Monday in Germany, fit or six weeks", () => {
    // read both locales' weeks and lay the month out
    const month = "2026-11-21";
    const us = US;
    const germany = GERMANY;

    expect({
        us: daysOf(MonthGrid.of(month, us, "fit")),
        germany: daysOf(MonthGrid.of(month, germany, "fit")),
        fixed: MonthGrid.of(month, us, "fixed").weeks.length,
        outside: ["2026-10-26", "2026-11-30"].map((date) =>
            MonthGrid.isOutside(MonthGrid.of(month, germany, "fit"), date),
        ),
    }).toEqual({
        us: [
            [1, 2, 3, 4, 5, 6, 7],
            [8, 9, 10, 11, 12, 13, 14],
            [15, 16, 17, 18, 19, 20, 21],
            [22, 23, 24, 25, 26, 27, 28],
            [29, 30, 1, 2, 3, 4, 5],
        ],
        germany: [
            [26, 27, 28, 29, 30, 31, 1],
            [2, 3, 4, 5, 6, 7, 8],
            [9, 10, 11, 12, 13, 14, 15],
            [16, 17, 18, 19, 20, 21, 22],
            [23, 24, 25, 26, 27, 28, 29],
            [30, 1, 2, 3, 4, 5, 6],
        ],
        fixed: 6,
        outside: [true, false],
    });
});
