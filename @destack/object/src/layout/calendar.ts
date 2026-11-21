import { type JsonObject, present, schema } from "@destack/schema";
import type { TimeScale } from "@destack/schema";

/** The milliseconds in a day, the step between two dates' midnights in UTC. */
const DAY_MILLISECONDS = 24 * 60 * 60 * 1000;

/** One day of a calendar in a time zone and the objects placed on it. */
export interface CalendarDay {
    /** The day's date in its time zone, as YYYY-MM-DD. */
    readonly date: string;
    /** The objects whose time falls on the day. */
    readonly rows: readonly JsonObject[];
}

/** One day of a calendar. */
export const CalendarDay = {
    /** Read the instant a day's date starts in UTC, which date formats read in UTC. */
    instant(day: CalendarDay): number {
        return Date.parse(`${day.date}T00:00:00Z`);
    },

    /** Read a day's ISO weekday, from 1 for Monday to 7 for Sunday. */
    weekday(day: CalendarDay): number {
        return new Date(CalendarDay.instant(day)).getUTCDay() || 7;
    },
};

/** Objects on the days of a time field. */
const calendarLayout = schema.object({
    /** The kind. */
    kind: schema.literal("calendar"),
    /** The time field placing the objects. */
    by: schema.string().min(1),
});
/** Objects on the days of a time field. */
export type CalendarLayout = schema.Infer<typeof calendarLayout>;

/** Objects on the days of a time field. */
export const CalendarLayout = Object.assign(calendarLayout, {
    /** List a window's days in a time zone, each with its objects. */
    days(
        layout: CalendarLayout,
        rows: readonly JsonObject[],
        timeZone: string,
        window?: TimeScale,
    ): CalendarDay[] {
        // read each object's date in the time zone
        const dateOf = dateFormat(timeZone);
        const dated = rows.flatMap((row) => {
            const time = row[layout.by];

            return typeof time === "number" ? [{ row, date: dateOf(time) }] : [];
        });

        // span the window's dates, else the objects' first to last date
        const sorted = dated.map((each) => each.date).toSorted();
        const first = window === undefined ? sorted.at(0) : dateOf(window.start);
        const last = window === undefined ? sorted.at(-1) : dateOf(window.end - 1);
        if (first === undefined || last === undefined) {
            return [];
        }

        // list each date with its objects
        const days: CalendarDay[] = [];
        for (let date = first; date <= last; date = nextDate(date)) {
            const placed = dated.filter((each) => each.date === date).map((each) => each.row);
            days.push({ date, rows: placed });
        }

        return days;
    },

    /** Read the window of the month a moment falls in, in a time zone. */
    month(now: number, timeZone: string): TimeScale {
        // take the month's first date and the next month's
        const today = dateFormat(timeZone)(now);
        const first = `${today.slice(0, 7)}-01`;
        const following = new Date(Date.parse(`${first}T00:00:00Z`));
        following.setUTCMonth(following.getUTCMonth() + 1);
        const after = following.toISOString().slice(0, 10);

        return { start: midnight(first, timeZone), end: midnight(after, timeZone) };
    },

    /** Extend a list query to read the objects within a window. */
    query<Query extends { readonly where?: object; readonly orderBy?: object }>(
        layout: CalendarLayout,
        query: Query,
        window?: TimeScale,
    ): Query {
        // read every object without a window
        if (window === undefined) {
            return query;
        }

        const within = { [layout.by]: { gte: window.start, lt: window.end } };

        return {
            ...query,
            where: query.where === undefined ? within : { AND: [query.where, within] },
        };
    },
});

/** Read moments' dates in a time zone, as YYYY-MM-DD. */
function dateFormat(timeZone: string): (time: number) => string {
    // format the year, month and day, which en-CA writes as YYYY-MM-DD
    const format = new Intl.DateTimeFormat("en-CA", {
        timeZone,
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
    });

    return (time) => format.format(time);
}

/** Read the date after a date. */
function nextDate(date: string): string {
    const next = Date.parse(`${date}T00:00:00Z`) + DAY_MILLISECONDS;

    return new Date(next).toISOString().slice(0, 10);
}

/** Read the moment a date starts in a time zone. */
function midnight(date: string, timeZone: string): number {
    // read the zone's wall clock at UTC midnight of the date
    const guess = Date.parse(`${date}T00:00:00Z`);
    const parts = new Intl.DateTimeFormat("en-US", {
        timeZone,
        hourCycle: "h23",
        year: "numeric",
        month: "numeric",
        day: "numeric",
        hour: "numeric",
        minute: "numeric",
        second: "numeric",
    }).formatToParts(guess);
    const part = (type: Intl.DateTimeFormatPartTypes) =>
        Number(
            present(
                parts.find((each) => each.type === type),
                type,
            ).value,
        );

    // shift by the zone's offset there
    const wall = Date.UTC(
        part("year"),
        part("month") - 1,
        part("day"),
        part("hour"),
        part("minute"),
        part("second"),
    );

    return guess - (wall - guess);
}
