import * as schema from "../validate/index.ts";
import { defineSchema } from "../declare/schema.ts";

/** The minutes in an hour. */
const HOUR_MINUTES = 60;

/** A weekday as ISO 8601 numbers it, Monday 1 through Sunday 7. */
export const Weekday = defineSchema(schema.number().int().min(1).max(7));
/** A weekday, Monday 1 through Sunday 7. */
export type Weekday = schema.Infer<typeof Weekday>;

/** A calendar date without a time zone, as Temporal's PlainDate. */
export interface PlainDate {
    /** The year. */
    readonly year: number;
    /** The month, January 1. */
    readonly month: number;
    /** The day of the month. */
    readonly day: number;
}

/** Calendar dates without a time zone. */
export const PlainDate = {
    /** Move a date by some days. */
    add(date: PlainDate, days: number): PlainDate {
        const moved = new Date(Date.UTC(date.year, date.month - 1, date.day + days));

        return {
            year: moved.getUTCFullYear(),
            month: moved.getUTCMonth() + 1,
            day: moved.getUTCDate(),
        };
    },

    /** Order two dates: negative when the left comes first, zero when they are the same day. */
    compare(left: PlainDate, right: PlainDate): number {
        return left.year - right.year || left.month - right.month || left.day - right.day;
    },

    /** Report whether two dates are the same day, false when either is absent. */
    equals(left: PlainDate | undefined, right: PlainDate | undefined): boolean {
        return left !== undefined && right !== undefined && PlainDate.compare(left, right) === 0;
    },

    /** Read the weekday of a date. */
    weekday(date: PlainDate): Weekday {
        const day = new Date(Date.UTC(date.year, date.month - 1, date.day)).getUTCDay();

        return day === 0 ? 7 : day;
    },
};

/** A time of day written HH:MM on a 24-hour clock, as Temporal's PlainTime without seconds. */
export const PlainTime = Object.assign(
    defineSchema(schema.string().regex(/^(?:[01]\d|2[0-3]):[0-5]\d$(?![\s\S])/u)),
    {
        /** Read the minutes since midnight of a time of day. */
        minutes(time: PlainTime): number {
            const hours = Number(time.slice(0, 2));
            const minutes = Number(time.slice(3, 5));

            return hours * HOUR_MINUTES + minutes;
        },
    },
);
/** A time of day, written HH:MM. */
export type PlainTime = schema.Infer<typeof PlainTime>;
