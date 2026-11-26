import type { Environment } from "@destack/environment";
import { type Localization, t } from "@destack/locale";
import { PlainDate, PlainTime, TimeZone } from "@destack/schema";
import {
    type DateSegmentFormat,
    type DateSegmentKind,
    type DateSegmentValues,
    isDateSegment,
} from "./date-segment.tsx";

/** The earliest and latest year a date field takes, as four digits write them. */
const YEARS = [1, 9999] as const;

/** The months of a year. */
const MONTHS = 12;

/** The months of 30 days, the others but February having 31. */
const SHORT_MONTHS: ReadonlySet<number> = new Set([4, 6, 9, 11]);

/** A date whose parts differ, which lays out a locale's date fields. */
const SAMPLE_DATE = Date.UTC(2026, 10, 22);

/** The hours of half a day. */
const HALF_DAY = 12;

/** The minutes of an hour. */
const HOUR_MINUTES = 60;

/** A time in the afternoon whose parts differ, which lays out a locale's time fields. */
const SAMPLE_TIME = Date.UTC(2026, 0, 1, 13, 45);

/** A time in the morning, whose day period is the morning. */
const MORNING = Date.UTC(2026, 0, 1, 1);

/** The day whose times are written. */
const SAMPLE_DAY = PlainDate.of(1970, 1, 1);

/** How many hours a clock counts to. */
export type HourCycle = 12 | 24;

/** A minute in milliseconds. */
const MINUTE = 60_000;

/** The year, month and day segments of a date in a locale's order, bounds and numerals. */
export class DateFormat implements DateSegmentFormat<PlainDate> {
    /** The segments and the literals between them in the locale's order. */
    readonly layout: readonly Intl.DateTimeFormatPart[];
    /** The locale the segments write and name themselves in. */
    readonly #locale: Localization;

    /** Lay out a locale's date segments in the Gregorian calendar. */
    constructor(locale: Localization) {
        const format = new Intl.DateTimeFormat(locale.tag, {
            year: "numeric",
            month: "2-digit",
            day: "2-digit",
            calendar: "gregory",
            timeZone: "UTC",
        });
        this.layout = format
            .formatToParts(SAMPLE_DATE)
            .filter((part) => isDateSegment(part) || part.type === "literal");
        this.#locale = locale;
    }

    /** Read the bounds of a segment, a day within its month once the month is known. */
    limits(kind: DateSegmentKind, values: DateSegmentValues): readonly [number, number] {
        // the four-digit years
        if (kind === "year") {
            return YEARS;
        }
        // the months of a year
        else if (kind === "month") {
            return [1, MONTHS];
        }

        return [1, daysIn(values.year, values.month)];
    }

    /** Write a year as its digits, and a month or day in two. */
    text(kind: DateSegmentKind, value: number): string {
        return this.#locale.number(value, {
            minimumIntegerDigits: kind === "year" ? 1 : 2,
            useGrouping: false,
        });
    }

    /** Write a month by its name and a year or day as its number. */
    valueText(kind: DateSegmentKind, value: number): string {
        return kind === "month"
            ? this.#locale.date(Date.UTC(2000, value - 1, 1), { month: "long", timeZone: "UTC" })
            : this.#locale.number(value, { useGrouping: false });
    }

    /** Label a segment by the part of the date it shows. */
    name(kind: DateSegmentKind): string {
        // the year
        if (kind === "year") {
            return this.#locale.render(t.context("label")`Year`);
        }
        // the month
        else if (kind === "month") {
            return this.#locale.render(t.context("label")`Month`);
        }

        return this.#locale.render(t.context("label")`Day`);
    }

    /** Read today's year, month or day. */
    now(kind: DateSegmentKind, environment: Pick<Environment, "clock" | "timeZone">): number {
        // read the part of today's date
        const today = TimeZone.date(environment.timeZone, environment.clock.now());
        if (kind === "year") {
            return PlainDate.year(today);
        }
        // the month
        else if (kind === "month") {
            return PlainDate.month(today);
        }

        return PlainDate.day(today);
    }

    /** Read the date of the segments, undefined while one is empty or the date does not exist. */
    valueOf(values: DateSegmentValues): PlainDate | undefined {
        // wait for every segment
        const { year, month, day } = values;
        if (year === undefined || month === undefined || day === undefined) {
            return undefined;
        }

        // refuse a date outside its year's months and its month's days
        const isReal =
            year >= YEARS[0] &&
            month >= 1 &&
            month <= MONTHS &&
            day >= 1 &&
            day <= daysIn(year, month);

        return isReal ? PlainDate.of(year, month, day) : undefined;
    }

    /** Split a date into the values of its segments, none for no date. */
    valuesOf(date: PlainDate | undefined): DateSegmentValues {
        return date === undefined
            ? {}
            : {
                  year: PlainDate.year(date),
                  month: PlainDate.month(date),
                  day: PlainDate.day(date),
              };
    }

    /** Order two dates. */
    compare(left: PlainDate, right: PlainDate): number {
        return PlainDate.compare(left, right);
    }

    /** Write a date as people read it. */
    label(date: PlainDate): string {
        return this.#locale.plainDate(date, { dateStyle: "medium" });
    }

    /** Write a date as a form submits it. */
    write(date: PlainDate): string {
        return date;
    }
}

/** The hour, minute and day period segments of a time. */
export class TimeFormat implements DateSegmentFormat<PlainTime> {
    /** The segments and the literals between them in the locale's order. */
    readonly layout: readonly Intl.DateTimeFormatPart[];
    /** Whether hours count 1 to 12 beside a day period, else 0 to 23. */
    readonly isTwelveHour: boolean;
    /** The cycle Intl writes its hours in. */
    readonly hourCycle: "h12" | "h23";
    /** The locale the segments write and name themselves in. */
    readonly #locale: Localization;
    /** The format that writes the day periods. */
    readonly #format: Intl.DateTimeFormat;

    /** Lay out a locale's time segments on its hour cycle. */
    constructor(locale: Localization, hours?: HourCycle) {
        // take the clock given, else the locale's, a 12-hour one showing a day period
        const cycle = new Intl.DateTimeFormat(locale.tag, {
            hour: "numeric",
            timeZone: "UTC",
        }).resolvedOptions().hourCycle;
        this.isTwelveHour = hours === undefined ? cycle === "h11" || cycle === "h12" : hours === 12;
        this.hourCycle = this.isTwelveHour ? "h12" : "h23";
        const format = new Intl.DateTimeFormat(locale.tag, {
            hour: "2-digit",
            minute: "2-digit",
            hourCycle: this.hourCycle,
            timeZone: "UTC",
        });

        // list the segments and the literals
        this.layout = format
            .formatToParts(SAMPLE_TIME)
            .filter((part) => isDateSegment(part) || part.type === "literal");
        this.#locale = locale;
        this.#format = format;
    }

    /** Read the bounds of a segment on the locale's hour cycle. */
    limits(kind: DateSegmentKind): readonly [number, number] {
        // the hours of the clock
        if (kind === "hour") {
            return this.isTwelveHour ? [1, HALF_DAY] : [0, 2 * HALF_DAY - 1];
        }
        // the minutes of an hour
        else if (kind === "minute") {
            return [0, HOUR_MINUTES - 1];
        }

        return [0, 1];
    }

    /** Write an hour or minute in two digits, and a day period as the locale writes it. */
    text(kind: DateSegmentKind, value: number): string {
        // write a number
        if (kind !== "dayPeriod") {
            return this.#locale.number(value, { minimumIntegerDigits: 2, useGrouping: false });
        }

        // label the morning or the afternoon
        const time = value === 0 ? MORNING : SAMPLE_TIME;
        const period = this.#format.formatToParts(time).find((part) => part.type === "dayPeriod");
        if (period === undefined) {
            throw new TypeError(`the clock of '${this.#locale.tag}' writes no day period`);
        }

        return period.value;
    }

    /** Write a segment's value as it shows. */
    valueText(kind: DateSegmentKind, value: number): string {
        return this.text(kind, value);
    }

    /** Label a segment by the part of the time it shows. */
    name(kind: DateSegmentKind): string {
        // the hour
        if (kind === "hour") {
            return this.#locale.render(t.context("label")`Hour`);
        }
        // the minute
        else if (kind === "minute") {
            return this.#locale.render(t.context("label")`Minute`);
        }

        return this.#locale.render(t.context("label")`AM/PM`);
    }

    /** Read the hour, minute or day period on an environment's clock in its time zone. */
    now(kind: DateSegmentKind, environment: Pick<Environment, "clock" | "timeZone">): number {
        // read the clock's hour on the locale's hour cycle
        const minutes = PlainTime.minutes(
            TimeZone.time(environment.timeZone, environment.clock.now()),
        );
        const hours = Math.floor(minutes / HOUR_MINUTES);
        const minute = minutes % HOUR_MINUTES;
        if (kind === "hour") {
            return this.isTwelveHour ? hours % HALF_DAY || HALF_DAY : hours;
        }
        // the minute
        else if (kind === "minute") {
            return minute;
        }

        return hours < HALF_DAY ? 0 : 1;
    }

    /** Read the time of the segments, undefined while one is empty. */
    valueOf(values: DateSegmentValues): PlainTime | undefined {
        // wait for every segment the locale shows
        const { hour, minute, dayPeriod } = values;
        const isPeriodMissing = this.isTwelveHour && dayPeriod === undefined;
        if (hour === undefined || minute === undefined || isPeriodMissing) {
            return undefined;
        }

        // count the hours of a 12-hour clock from midnight
        const hours = dayPeriod === undefined ? hour : (hour % HALF_DAY) + HALF_DAY * dayPeriod;

        return `${pad(hours)}:${pad(minute)}`;
    }

    /** Split a time into the values of its segments on the locale's hour cycle. */
    valuesOf(time: PlainTime | undefined): DateSegmentValues {
        // leave every segment empty without a time
        if (time === undefined) {
            return {};
        }

        // split the hours into a 12-hour clock's hour and day period
        const minutes = PlainTime.minutes(time);
        const hours = Math.floor(minutes / HOUR_MINUTES);
        const minute = minutes % HOUR_MINUTES;

        return this.isTwelveHour
            ? { hour: hours % HALF_DAY || HALF_DAY, minute, dayPeriod: hours < HALF_DAY ? 0 : 1 }
            : { hour: hours, minute };
    }

    /** Order two times of a day. */
    compare(left: PlainTime, right: PlainTime): number {
        return PlainTime.minutes(left) - PlainTime.minutes(right);
    }

    /** Write a time as people read it. */
    label(time: PlainTime): string {
        return this.#format.format(TimeZone.instant("UTC", SAMPLE_DAY, time));
    }

    /** Write a time as a form submits it. */
    write(time: PlainTime): string {
        return time;
    }
}

/** The segments of an instant: its date's and its time's, in a zone. */
export class DateTimeFormat implements DateSegmentFormat<number> {
    /** The segments and the literals between them in the locale's order. */
    readonly layout: readonly Intl.DateTimeFormatPart[];
    /** The segments of its date. */
    readonly #date: DateFormat;
    /** The segments of its time of day. */
    readonly #time: TimeFormat;
    /** The zone its date and time are in. */
    readonly #timeZone: () => string;
    /** The locale it is written in. */
    readonly #locale: Localization;

    /** Lay out a locale's date and time segments. */
    constructor(locale: Localization, timeZone: () => string, hourCycle?: HourCycle) {
        // read the date and the time alone
        this.#date = new DateFormat(locale);
        this.#time = new TimeFormat(locale, hourCycle);
        this.#timeZone = timeZone;
        this.#locale = locale;

        // lay out both in the locale's order, the time first where it is
        this.layout = new Intl.DateTimeFormat(locale.tag, {
            year: "numeric",
            month: "2-digit",
            day: "2-digit",
            hour: "2-digit",
            minute: "2-digit",
            hourCycle: this.#time.hourCycle,
            calendar: "gregory",
            timeZone: "UTC",
        })
            .formatToParts(SAMPLE_DATE + SAMPLE_TIME)
            .filter((part) => isDateSegment(part) || part.type === "literal");
    }

    /** Read the bounds of a segment, a day within its month once the month is known. */
    limits(kind: DateSegmentKind, values: DateSegmentValues): readonly [number, number] {
        return this.#of(kind).limits(kind, values);
    }

    /** Write a segment's value as its date or its time does. */
    text(kind: DateSegmentKind, value: number): string {
        return this.#of(kind).text(kind, value);
    }

    /** Write a segment's value for assistive technology. */
    valueText(kind: DateSegmentKind, value: number): string {
        return this.#of(kind).valueText(kind, value);
    }

    /** Label a segment by the part of the date or the time it shows. */
    name(kind: DateSegmentKind): string {
        return this.#of(kind).name(kind);
    }

    /** Read a part of the date or the time now, in the zone. */
    now(kind: DateSegmentKind, environment: Pick<Environment, "clock" | "timeZone">): number {
        const { clock } = environment;

        return this.#of(kind).now(kind, { clock, timeZone: this.#timeZone() });
    }

    /** Read the instant of the segments, undefined while one is empty. */
    valueOf(values: DateSegmentValues): number | undefined {
        // wait for its date and its time
        const date = this.#date.valueOf(values);
        const time = this.#time.valueOf(values);

        return date === undefined || time === undefined
            ? undefined
            : TimeZone.instant(this.#timeZone(), date, time);
    }

    /** Split an instant into the values of its segments, none for no instant. */
    valuesOf(instant: number | undefined): DateSegmentValues {
        // leave every segment empty without an instant
        if (instant === undefined) {
            return {};
        }

        return {
            ...this.#date.valuesOf(TimeZone.date(this.#timeZone(), instant)),
            ...this.#time.valuesOf(TimeZone.time(this.#timeZone(), instant)),
        };
    }

    /** Order two instants by their minutes, which the segments show. */
    compare(left: number, right: number): number {
        return Math.floor(left / MINUTE) - Math.floor(right / MINUTE);
    }

    /** Write an instant as people read it, in the zone. */
    label(instant: number): string {
        return this.#locale.date(instant, {
            dateStyle: "medium",
            timeStyle: "short",
            hourCycle: this.#time.hourCycle,
            timeZone: this.#timeZone(),
        });
    }

    /** Write an instant as a form submits it. */
    write(instant: number): string {
        return new Date(instant).toISOString();
    }

    /** Find the format of a segment. */
    #of(kind: DateSegmentKind): DateFormat | TimeFormat {
        return kind === "year" || kind === "month" || kind === "day" ? this.#date : this.#time;
    }
}

/** Count the days of a month, the most any such month has while the year or month is unknown. */
function daysIn(year: number | undefined, month: number | undefined): number {
    // a month of 30 days, else February by the Gregorian leap rule, else 31
    if (month !== undefined && SHORT_MONTHS.has(month)) {
        return 30;
    } else if (month === 2) {
        const isLeap =
            year === undefined || (year % 4 === 0 && year % 100 !== 0) || year % 400 === 0;

        return isLeap ? 29 : 28;
    }

    return 31;
}

/** Write a part of a time in two digits. */
function pad(part: number): string {
    return String(part).padStart(2, "0");
}
