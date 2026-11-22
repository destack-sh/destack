import * as schema from "../validate/index.ts";

/** The minutes in an hour. */
const HOUR_MINUTES = 60;

/** The minutes in a day of the wall clock. */
const DAY_MINUTES = 24 * HOUR_MINUTES;

/** The days of a week. */
export const WEEK_DAYS = 7;

/** The months of a year. */
const YEAR_MONTHS = 12;

/** The milliseconds of a UTC day. */
const UTC_DAY_MILLISECONDS = 86_400_000;

/** The weekday of Thursday, the day that sets the year of an ISO 8601 week. */
const THURSDAY = 4;

/** A valid date written YYYY-MM-DD. */
const DATE_TEXT =
    /^(?:\d{4}-(?:(?:0[13578]|1[02])-(?:0[1-9]|[12]\d|3[01])|(?:0[469]|11)-(?:0[1-9]|[12]\d|30)|02-(?:0[1-9]|1\d|2[0-8]))|(?:\d{2}(?:0[48]|[2468][048]|[13579][26])|(?:[02468][048]|[13579][26])00)-02-29)$/u;

/** A weekday as ISO 8601 numbers it, Monday 1 through Sunday 7. */
export const Weekday = schema.number().int().min(1).max(7);
/** A weekday, Monday 1 through Sunday 7. */
export type Weekday = schema.Infer<typeof Weekday>;

/** A move of a date in calendar units. */
export const Period = schema.object({
    /** Years. */
    years: schema.number().int().exactOptional(),
    /** Months. */
    months: schema.number().int().exactOptional(),
    /** Weeks of seven days. */
    weeks: schema.number().int().exactOptional(),
    /** Days. */
    days: schema.number().int().exactOptional(),
});
/** A move of a date in calendar units. */
export type Period = Readonly<schema.Infer<typeof Period>>;

/** The schema of a calendar date. */
const plainDateSchema = schema.string().regex(DATE_TEXT);
/** A calendar date without a time zone, written YYYY-MM-DD. */
export type PlainDate = schema.Infer<typeof plainDateSchema>;

/** A calendar date without a time zone, written YYYY-MM-DD. */
export const PlainDate = Object.assign(plainDateSchema, {
    /** Write the date of a year, a month and a day. */
    of(year: number, month: number, day: number): PlainDate {
        const date = `${pad(year, 4)}-${pad(month, 2)}-${pad(day, 2)}`;
        if (!DATE_TEXT.test(date)) {
            throw new RangeError(`not a date: '${date}'`);
        }

        return date;
    },

    /** Read a date's year. */
    year(date: PlainDate): number {
        return Number(date.slice(0, 4));
    },

    /** Read a date's month, January 1. */
    month(date: PlainDate): number {
        return Number(date.slice(5, 7));
    },

    /** Read a date's day of the month. */
    day(date: PlainDate): number {
        return Number(date.slice(8, 10));
    },

    /** Move a date by a period, months before days. */
    add(date: PlainDate, period: Period): PlainDate {
        // move by whole months and clamp the day to the month
        const { years = 0, months = 0, weeks = 0, days = 0 } = period;
        const index =
            (PlainDate.year(date) + years) * YEAR_MONTHS + (PlainDate.month(date) - 1) + months;
        const year = Math.floor(index / YEAR_MONTHS);
        const month = (((index % YEAR_MONTHS) + YEAR_MONTHS) % YEAR_MONTHS) + 1;
        const day = Math.min(PlainDate.day(date), daysIn(year, month));

        // then move by days in UTC
        return fromUtc(utcOf(year, month, day) + (weeks * WEEK_DAYS + days) * UTC_DAY_MILLISECONDS);
    },

    /** Replace fields of a date and clamp the day to the month. */
    with(
        date: PlainDate,
        fields: { readonly year?: number; readonly month?: number; readonly day?: number },
    ): PlainDate {
        // default each field to the date's
        const year = fields.year ?? PlainDate.year(date);
        const month = fields.month ?? PlainDate.month(date);
        const day = fields.day ?? PlainDate.day(date);

        return PlainDate.of(year, month, Math.min(day, daysIn(year, month)));
    },

    /** Count the days from one date to another. */
    daysUntil(from: PlainDate, to: PlainDate): number {
        return (toUtc(to) - toUtc(from)) / UTC_DAY_MILLISECONDS;
    },

    /** Order two dates: negative when the left comes first, zero when they are the same day. */
    compare(left: PlainDate, right: PlainDate): number {
        return left === right ? 0 : left < right ? -1 : 1;
    },

    /** Report whether two dates are the same day. */
    equals(left: PlainDate | undefined, right: PlainDate | undefined): boolean {
        return left !== undefined && left === right;
    },

    /** Clamp a date between an earliest and a latest date. */
    clamp(
        date: PlainDate,
        earliest: PlainDate | undefined,
        latest: PlainDate | undefined,
    ): PlainDate {
        if (earliest !== undefined && date < earliest) {
            return earliest;
        }

        return latest !== undefined && date > latest ? latest : date;
    },

    /** Read the weekday of a date, Monday 1 through Sunday 7. */
    dayOfWeek(date: PlainDate): Weekday {
        const day = new Date(toUtc(date)).getUTCDay();

        return day === 0 ? WEEK_DAYS : day;
    },

    /** Read the days of a date's month. */
    daysInMonth(date: PlainDate): number {
        return daysIn(PlainDate.year(date), PlainDate.month(date));
    },

    /** Read the ISO 8601 week number of a date. */
    weekOfYear(date: PlainDate): number {
        // count the weeks to the Thursday of the date's week
        const thursday = PlainDate.add(date, { days: THURSDAY - PlainDate.dayOfWeek(date) });
        const days = PlainDate.daysUntil(PlainDate.of(PlainDate.year(thursday), 1, 1), thursday);

        return Math.floor(days / WEEK_DAYS) + 1;
    },
});

/** A time of day written HH:MM on a 24-hour clock, as Temporal's PlainTime without seconds. */
export const PlainTime = Object.assign(schema.string().regex(/^(?:[01]\d|2[0-3]):[0-5]\d$/u), {
    /** Read the minutes since midnight of a time of day. */
    minutes(time: PlainTime): number {
        const hours = Number(time.slice(0, 2));
        const minutes = Number(time.slice(3, 5));

        return hours * HOUR_MINUTES + minutes;
    },

    /** Write the time of day some minutes after midnight, within one day. */
    of(minutes: number): string {
        // refuse minutes outside the day
        if (!Number.isInteger(minutes) || minutes < 0 || minutes >= DAY_MINUTES) {
            throw new RangeError(
                `time of day is invalid: '${String(minutes)}' minutes after midnight`,
            );
        }
        const hours = String(Math.floor(minutes / HOUR_MINUTES)).padStart(2, "0");
        const rest = String(minutes % HOUR_MINUTES).padStart(2, "0");

        return `${hours}:${rest}`;
    },
});
/** A time of day, written HH:MM. */
export type PlainTime = schema.Infer<typeof PlainTime>;

/** Read a date's midnight in UTC. */
function toUtc(date: PlainDate): number {
    return utcOf(PlainDate.year(date), PlainDate.month(date), PlainDate.day(date));
}

/** Read the UTC instant of a year, a month and a day. */
function utcOf(year: number, month: number, day: number): number {
    // set the year apart to keep years below 100
    return new Date(0).setUTCFullYear(year, month - 1, day);
}

/** Read the date of a UTC instant. */
function fromUtc(time: number): PlainDate {
    const moved = new Date(time);

    return PlainDate.of(moved.getUTCFullYear(), moved.getUTCMonth() + 1, moved.getUTCDate());
}

/** Read the days of a month of a year. */
function daysIn(year: number, month: number): number {
    return new Date(utcOf(year, month + 1, 0)).getUTCDate();
}

/** Write a number with leading zeros to a width. */
function pad(value: number, width: number): string {
    return String(value).padStart(width, "0");
}
