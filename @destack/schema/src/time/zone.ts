import * as schema from "../validate/index.ts";
import { defineSchema } from "../inspect/schema.ts";
import { PlainDate, PlainTime } from "./plain.ts";

/** The longest IANA time zone name kept, beyond the longest in the database. */
const TIME_ZONE_LENGTH = 64;

/** The milliseconds in a minute. */
const MINUTE_MILLISECONDS = 60_000;

/** The date formatters by time zone, since each takes tens of microseconds to build. */
const formatters = new Map<string, Intl.DateTimeFormat>();

/** The schema of an IANA time zone name. */
const timeZoneSchema = defineSchema(
    schema
        .string()
        .max(TIME_ZONE_LENGTH)
        .regex(/^[A-Za-z][A-Za-z0-9_+-]*(?:\/[A-Za-z0-9_+-]+)*$/),
);
/** An IANA time zone name. */
export type TimeZone = schema.Infer<typeof timeZoneSchema>;

/** An IANA time zone name, such as Europe/Vienna. */
export const TimeZone = Object.assign(timeZoneSchema, {
    /** Require a time zone this runtime knows, its aliases included. */
    require(zone: TimeZone): void {
        new Intl.DateTimeFormat("en", { timeZone: zone });
    },

    /** Read the date an instant falls on in a time zone. */
    date(zone: TimeZone, at: number): PlainDate {
        const parts = formatter(zone).formatToParts(at);
        const part = (type: Intl.DateTimeFormatPartTypes) =>
            Number(parts.find((entry) => entry.type === type)!.value);

        return { year: part("year"), month: part("month"), day: part("day") };
    },

    /** Read the instant a date's time of day occurs in a time zone, moving a time in a gap past it. */
    instant(zone: TimeZone, date: PlainDate, time: PlainTime): number {
        // correct the wall time by its offset twice, across offset changes
        const wall = Date.UTC(date.year, date.month - 1, date.day, 0, PlainTime.minutes(time));
        const guess = wall - offset(zone, wall);

        return wall - offset(zone, guess);
    },

    /** Read the next instant one of some times of day occurs in a time zone. */
    next(zone: TimeZone, times: readonly PlainTime[], now: number): number {
        // try each time today and tomorrow
        const today = TimeZone.date(zone, now);
        const candidates = [0, 1].flatMap((days) =>
            times.map((time) => TimeZone.instant(zone, PlainDate.add(today, days), time)),
        );
        const next = Math.min(...candidates.filter((candidate) => candidate > now));
        if (!Number.isFinite(next)) {
            throw new TypeError("next time of day needs at least one time");
        }

        return next;
    },
});

/** Read the offset of a time zone from UTC at an instant, in milliseconds. */
function offset(zone: TimeZone, at: number): number {
    // read the local wall time as UTC
    const parts = formatter(zone).formatToParts(at);
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

/** Read the formatter of a time zone, refusing an unknown zone. */
function formatter(zone: TimeZone): Intl.DateTimeFormat {
    let known = formatters.get(zone);
    if (known === undefined) {
        known = new Intl.DateTimeFormat("en-US", {
            timeZone: zone,
            hourCycle: "h23",
            year: "numeric",
            month: "numeric",
            day: "numeric",
            hour: "numeric",
            minute: "numeric",
        });
        formatters.set(zone, known);
    }

    return known;
}
