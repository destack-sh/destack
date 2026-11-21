import { expect, test } from "@destack/test";
import { PlainDate } from "./plain.ts";
import { Week } from "./week.ts";

/** The week of the United States: Sunday first. */
const US: Week = { firstDay: 7, weekend: [6, 7], minimalDays: 1 };

/** The week of Germany: Monday first, numbered as ISO 8601. */
const GERMANY: Week = { firstDay: 1, weekend: [6, 7], minimalDays: 4 };

/** The week of Saudi Arabia: Sunday first, with Friday and Saturday as its weekend. */
const SAUDI_ARABIA: Week = { firstDay: 7, weekend: [5, 6], minimalDays: 1 };

test("number weeks by the locale: ISO 8601 in Germany, the week of January first in the US", () => {
    // read the weeks around the turn of 2026 to 2027 in both locales
    const dates = ["2026-01-01", "2026-12-31", "2027-01-01", "2027-01-04"];
    const germany = GERMANY;
    const us = US;

    // germany matches ISO 8601, the US starts week one with the week of January first
    expect({
        germany: dates.map((date) => Week.weekOfYear(germany, date)),
        iso: dates.map((date) => PlainDate.weekOfYear(date)),
        us: dates.map((date) => Week.weekOfYear(us, date)),
        weekend: ["2026-11-20", "2026-11-21"].map((text) => Week.isWeekend(SAUDI_ARABIA, text)),
    }).toEqual({
        germany: [1, 53, 53, 1],
        iso: [1, 53, 53, 1],
        us: [1, 1, 1, 2],
        weekend: [true, true],
    });
});
