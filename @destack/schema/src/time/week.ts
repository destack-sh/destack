import { PlainDate, WEEK_DAYS, type Weekday } from "./plain.ts";

/** The week of a locale. */
export interface Week {
    /** The first day of the week. */
    readonly firstDay: Weekday;
    /** The days of the weekend. */
    readonly weekend: readonly Weekday[];
    /** The fewest days of the first week of a year. */
    readonly minimalDays: number;
}

/** The week of a locale. */
export const Week = {
    /** Move a date back to the start of its week. */
    startOf(week: Week, date: PlainDate): PlainDate {
        const offset = (PlainDate.dayOfWeek(date) - week.firstDay + WEEK_DAYS) % WEEK_DAYS;

        return PlainDate.add(date, { days: -offset });
    },

    /** Report whether a date falls on the weekend. */
    isWeekend(week: Week, date: PlainDate): boolean {
        return week.weekend.includes(PlainDate.dayOfWeek(date));
    },

    /** Read the week number of a date. */
    weekOfYear(week: Week, date: PlainDate): number {
        // find week one of this year, the next and the last
        const start = Week.startOf(week, date);
        const next = firstWeekOf(week, PlainDate.year(date) + 1);
        const current = firstWeekOf(week, PlainDate.year(date));
        const first =
            PlainDate.compare(start, next) >= 0
                ? next
                : PlainDate.compare(start, current) >= 0
                  ? current
                  : firstWeekOf(week, PlainDate.year(date) - 1);

        return PlainDate.daysUntil(first, start) / WEEK_DAYS + 1;
    },
};

/** Find the start of the first week of a year. */
function firstWeekOf(week: Week, year: number): PlainDate {
    // take the week of January first, or the week after
    const january = PlainDate.of(year, 1, 1);
    const start = Week.startOf(week, january);
    const days = WEEK_DAYS - PlainDate.daysUntil(start, january);

    return days >= week.minimalDays ? start : PlainDate.add(start, { weeks: 1 });
}
