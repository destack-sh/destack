import { defineSchema, schema } from "@destack/schema";

/** The milliseconds in a minute. */
const MINUTE_MILLISECONDS = 60_000;

/** A time of day, written HH:MM on a 24-hour clock. */
export const LocalTime = defineSchema(
    schema.string().regex(/^(?:[01]\d|2[0-3]):[0-5]\d$(?![\s\S])/),
);
/** A time of day, written HH:MM. */
export type LocalTime = schema.Infer<typeof LocalTime>;

/** A weekday, Monday 1 through Sunday 7. */
export const Weekday = defineSchema(schema.number().int().min(1).max(7));

/** A recurring span of local time on some weekdays. */
export const Window = defineSchema(
    schema.object({
        /** The weekdays it starts on. */
        days: schema.array(Weekday).min(1).max(7),
        /** The time it starts. */
        from: LocalTime,
        /** The time it ends, the next day when not after the start. */
        to: LocalTime,
    }),
);
/** A recurring span of local time on some weekdays. */
export type Window = schema.Infer<typeof Window>;

/** A calendar date in a time zone. */
interface LocalDate {
    /** The year. */
    readonly year: number;
    /** The month, January 1. */
    readonly month: number;
    /** The day of the month. */
    readonly day: number;
}

/** The date formatters by time zone, since each takes tens of microseconds to build. */
const formatters = new Map<string, Intl.DateTimeFormat>();

/** Instants in IANA time zones. */
export const TimeZone = {
    /** Read the next instant one of the local times occurs. */
    next(timeZone: string, times: readonly LocalTime[], now: number): number {
        // try each time today and tomorrow
        const today = localDate(timeZone, now);
        const candidates = [0, 1].flatMap((offset) =>
            times.map((time) => instant(timeZone, shift(today, offset), minutes(time))),
        );
        const next = Math.min(...candidates.filter((candidate) => candidate > now));
        if (!Number.isFinite(next)) {
            throw new TypeError("next local time needs at least one time");
        }

        return next;
    },

    /** Read when the windows holding now end, absent outside them. */
    end(timeZone: string, windows: readonly Window[], now: number): number | undefined {
        // check windows starting yesterday and today
        const today = localDate(timeZone, now);
        const ends: number[] = [];
        for (const offset of [-1, 0]) {
            const date = shift(today, offset);
            for (const window of windows) {
                // skip windows not starting on the date's weekday
                if (!window.days.includes(weekday(date))) {
                    continue;
                }

                // check that now lies between start and end
                const from = minutes(window.from);
                const to = minutes(window.to);
                const start = instant(timeZone, date, from);
                const stop = instant(timeZone, shift(date, to <= from ? 1 : 0), to);
                if (start <= now && now < stop) {
                    ends.push(stop);
                }
            }
        }

        return ends.length === 0 ? undefined : Math.max(...ends);
    },
};

/** Read the minutes since midnight of a local time. */
function minutes(time: LocalTime): number {
    const hours = Number(time.slice(0, 2));
    const rest = Number(time.slice(3, 5));

    return hours * 60 + rest;
}

/** Read the date an instant falls on in a time zone. */
function localDate(timeZone: string, now: number): LocalDate {
    const parts = formatter(timeZone).formatToParts(now);
    const part = (type: Intl.DateTimeFormatPartTypes) =>
        Number(parts.find((entry) => entry.type === type)!.value);

    return { year: part("year"), month: part("month"), day: part("day") };
}

/** Read the offset of a time zone from UTC at an instant, in milliseconds. */
function offset(timeZone: string, at: number): number {
    // read the local wall time as UTC
    const parts = formatter(timeZone).formatToParts(at);
    const part = (type: Intl.DateTimeFormatPartTypes) =>
        Number(parts.find((entry) => entry.type === type)!.value);
    const wall = Date.UTC(
        part("year"),
        part("month") - 1,
        part("day"),
        part("hour"),
        part("minute"),
    );
    const minute = at - (at % MINUTE_MILLISECONDS);

    return wall - minute;
}

/** Read the instant of a local date and minute in a time zone. */
function instant(timeZone: string, date: LocalDate, minute: number): number {
    // correct the wall time by its offset twice, across offset changes
    const wall = Date.UTC(date.year, date.month - 1, date.day, 0, minute);
    const guess = wall - offset(timeZone, wall);

    return wall - offset(timeZone, guess);
}

/** Move a date by some days. */
function shift(date: LocalDate, days: number): LocalDate {
    const moved = new Date(Date.UTC(date.year, date.month - 1, date.day + days));

    return {
        year: moved.getUTCFullYear(),
        month: moved.getUTCMonth() + 1,
        day: moved.getUTCDate(),
    };
}

/** Read the weekday of a date, Monday 1 through Sunday 7. */
function weekday(date: LocalDate): number {
    const day = new Date(Date.UTC(date.year, date.month - 1, date.day)).getUTCDay();

    return day === 0 ? 7 : day;
}

/** Read the formatter of a time zone, refusing an unknown zone. */
function formatter(timeZone: string): Intl.DateTimeFormat {
    let known = formatters.get(timeZone);
    if (known === undefined) {
        known = new Intl.DateTimeFormat("en-US", {
            timeZone,
            hourCycle: "h23",
            year: "numeric",
            month: "numeric",
            day: "numeric",
            hour: "numeric",
            minute: "numeric",
        });
        formatters.set(timeZone, known);
    }

    return known;
}
