import type { LocaleTag } from "@destack/locale";
import { PlainDate } from "@destack/schema";

/** The first day of the week where the locale does not say, ISO 8601's Monday. */
const DEFAULT_FIRST_DAY = 1;

/** The days of a week. */
const WEEK = 7;

/** Calendar days as plain dates without time zones, read, stepped and written in one place. */
export const Day = {
    /** Make the date of a year, month from 1 and day of month, overflowing into the next months. */
    of(year: number, month: number, day: number): PlainDate {
        return PlainDate.add({ year, month, day: 1 }, day - 1);
    },

    /** Read a `YYYY-MM-DD` date, as ISO 8601 and HTML date inputs write it. */
    parse(text: string): PlainDate {
        const match = /^(\d{4})-(\d{2})-(\d{2})$/u.exec(text);
        if (match === null) {
            throw new RangeError(`not a date: ${text}`);
        }

        return Day.of(Number(match[1]), Number(match[2]), Number(match[3]));
    },

    /** Write a date as `YYYY-MM-DD`. */
    key(date: PlainDate): string {
        // pad each part with leading zeros
        const year = String(date.year).padStart(4, "0");
        const month = String(date.month).padStart(2, "0");
        const day = String(date.day).padStart(2, "0");

        return `${year}-${month}-${day}`;
    },

    /** Read the UTC midnight of a date, for formatting it in the UTC time zone. */
    time(date: PlainDate): number {
        return Date.UTC(date.year, date.month - 1, date.day);
    },

    /** Read the date of the local clock now. */
    today(): PlainDate {
        const now = new Date();

        return { year: now.getFullYear(), month: now.getMonth() + 1, day: now.getDate() };
    },

    /** Move a date by a number of months, keeping the day of month within the target month. */
    addMonths(date: PlainDate, months: number): PlainDate {
        // find the target month and its last day
        const first = Day.of(date.year, date.month + months, 1);
        const last = PlainDate.add(Day.of(first.year, first.month + 1, 1), -1).day;

        return { ...first, day: Math.min(date.day, last) };
    },

    /** Read the first day of a date's month. */
    monthOf(date: PlainDate): PlainDate {
        return { year: date.year, month: date.month, day: 1 };
    },

    /** Read the first day of the week a locale uses, else Monday. */
    firstDayOf(tag: LocaleTag): number {
        const locale = new Intl.Locale(tag);

        return typeof locale.getWeekInfo === "function"
            ? locale.getWeekInfo().firstDay
            : DEFAULT_FIRST_DAY;
    },

    /** Move a date to the start of its week, the week starting on a weekday from 1 to 7. */
    startOfWeek(date: PlainDate, firstDay: number): PlainDate {
        const offset = (PlainDate.weekday(date) - firstDay + WEEK) % WEEK;

        return PlainDate.add(date, -offset);
    },

    /** List the weeks a month's grid shows, from the week of its first day to the week of its last. */
    weeks(month: PlainDate, firstDay: number): PlainDate[][] {
        // span whole weeks from the month's first day to its last
        const first = Day.monthOf(month);
        const last = PlainDate.add(Day.addMonths(first, 1), -1);
        const start = Day.startOfWeek(first, firstDay);
        const end = PlainDate.add(Day.startOfWeek(last, firstDay), WEEK - 1);

        // cut the days into weeks
        const weeks: PlainDate[][] = [];
        for (let day = start; PlainDate.compare(day, end) <= 0; day = PlainDate.add(day, WEEK)) {
            weeks.push(Array.from({ length: WEEK }, (_, index) => PlainDate.add(day, index)));
        }

        return weeks;
    },
};
