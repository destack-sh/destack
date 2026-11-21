import { PlainDate, WEEK_DAYS } from "./plain.ts";
import { Week } from "./week.ts";

/** The weeks of a fixed month grid. */
const FIXED_WEEKS = 6;

/** How many weeks a month grid shows. */
export type MonthRows = "fit" | "fixed";

/** A month laid out in whole weeks. */
export interface MonthGrid {
    /** The first day of the month. */
    readonly month: PlainDate;
    /** The weeks, each of seven days. */
    readonly weeks: readonly (readonly PlainDate[])[];
}

/** Months laid out in whole weeks. */
export const MonthGrid = {
    /** Lay a date's month out in whole weeks. */
    of(date: PlainDate, week: Week, rows: MonthRows): MonthGrid {
        // start at the week of the first day of the date's month
        const first = PlainDate.with(date, { day: 1 });
        const last = PlainDate.with(date, { day: PlainDate.daysInMonth(date) });
        const start = Week.startOf(week, first);

        // count the weeks to the last day, or six
        const touched = PlainDate.daysUntil(start, Week.startOf(week, last)) / WEEK_DAYS + 1;
        const count = rows === "fixed" ? FIXED_WEEKS : touched;

        return {
            month: first,
            weeks: Array.from({ length: count }, (_, row) =>
                Array.from({ length: WEEK_DAYS }, (__, column) =>
                    PlainDate.add(start, { days: row * WEEK_DAYS + column }),
                ),
            ),
        };
    },

    /** Report whether a date of the grid lies outside its month. */
    isOutside(grid: MonthGrid, date: PlainDate): boolean {
        return date.slice(0, 7) !== grid.month.slice(0, 7);
    },
};
