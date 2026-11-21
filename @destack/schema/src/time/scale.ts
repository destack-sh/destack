import { type Period, PlainDate } from "./plain.ts";
import { Week } from "./week.ts";
import { TimeZone } from "./zone.ts";

/** The milliseconds of each fixed tick step. */
const FIXED_STEPS = [
    1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 5000, 15_000, 30_000, 60_000, 300_000, 900_000,
    1_800_000, 3_600_000, 10_800_000, 21_600_000, 43_200_000,
] as const;

/** The milliseconds of a minute. */
const MINUTE = 60_000;

/** The milliseconds of an hour. */
const HOUR = 3_600_000;

/** The calendar steps beyond hours. */
const CALENDAR_STEPS = [
    { unit: "day", count: 1 },
    { unit: "day", count: 2 },
    { unit: "week", count: 1 },
    { unit: "month", count: 1 },
    { unit: "month", count: 3 },
    { unit: "year", count: 1 },
] as const;

/** The approximate milliseconds of each calendar unit. */
const UNIT_LENGTHS = {
    day: 86_400_000,
    week: 604_800_000,
    month: 2_629_746_000,
    year: 31_556_952_000,
} as const;

/** The unit of a tick. */
export type TickUnit =
    | "millisecond"
    | "second"
    | "minute"
    | "hour"
    | "day"
    | "week"
    | "month"
    | "year";

/** A mark on a time axis. */
export interface Tick {
    /** The time, in epoch milliseconds. */
    readonly at: number;
    /** The unit the step between ticks counts in. */
    readonly unit: TickUnit;
}

/** A window of time along an axis. */
export interface TimeScale {
    /** The time at the axis's start, in epoch milliseconds. */
    readonly start: number;
    /** The time at the axis's end, in epoch milliseconds. */
    readonly end: number;
}

/** A window of time along an axis. */
export const TimeScale = {
    /** Read where a time falls along the axis, from 0 to 1. */
    at(scale: TimeScale, time: number): number {
        return (time - scale.start) / (scale.end - scale.start);
    },

    /** Read the time at a place along the axis. */
    invert(scale: TimeScale, place: number): number {
        return scale.start + place * (scale.end - scale.start);
    },

    /** Zoom by a factor around a place along the axis. */
    zoom(scale: TimeScale, factor: number, around: number): TimeScale {
        // zoom around the time at the place
        const pivot = TimeScale.invert(scale, around);
        const length = (scale.end - scale.start) / factor;

        return { start: pivot - around * length, end: pivot + (1 - around) * length };
    },

    /** Move the window by a share of its length. */
    pan(scale: TimeScale, share: number): TimeScale {
        const offset = share * (scale.end - scale.start);

        return { start: scale.start + offset, end: scale.end + offset };
    },

    /** Mark the axis about a count of times in a time zone. */
    ticks(scale: TimeScale, count: number, timeZone: TimeZone, week: Week): Tick[] {
        // take the first step long enough for the count
        const target = TimeScale.step(scale, count);
        const fixed = FIXED_STEPS.find((step) => step >= target);
        if (fixed !== undefined) {
            return fixedTicks(scale, fixed, timeZone);
        }
        const calendar = CALENDAR_STEPS.find(
            (step) => step.count * UNIT_LENGTHS[step.unit] >= target,
        );

        // count whole years past the longest calendar step
        return calendar === undefined
            ? calendarTicks(scale, "year", Math.ceil(target / UNIT_LENGTHS.year), timeZone, week)
            : calendarTicks(scale, calendar.unit, calendar.count, timeZone, week);
    },

    /** Read the shortest step for a count of ticks. */
    step(scale: TimeScale, count: number): number {
        return (scale.end - scale.start) / Math.max(count, 1);
    },

    /** Move an instant to the nearest mark of the axis. */
    snap(scale: TimeScale, count: number, at: number, timeZone: TimeZone, week: Week): number {
        // take the nearest mark around the instant
        const step = TimeScale.step(scale, count);
        const around = { start: at - 2 * step, end: at + 2 * step };
        const marks = TimeScale.ticks(around, 4, timeZone, week);

        return marks.reduce((nearest, tick) =>
            Math.abs(tick.at - at) < Math.abs(nearest.at - at) ? tick : nearest,
        ).at;
    },
};

/** Mark a window at a fixed step. */
function fixedTicks(scale: TimeScale, step: number, timeZone: TimeZone): Tick[] {
    // align hour steps to local hours
    const offset = step >= HOUR ? TimeZone.offset(timeZone, scale.start) : 0;
    const first = Math.ceil((scale.start + offset) / step) * step - offset;
    const unit = unitOf(step);

    // step through the window
    const ticks: Tick[] = [];
    for (let at = first; at <= scale.end; at += step) {
        ticks.push({ at, unit });
    }

    return ticks;
}

/** Mark a window at calendar steps. */
function calendarTicks(
    scale: TimeScale,
    unit: "day" | "week" | "month" | "year",
    count: number,
    timeZone: TimeZone,
    week: Week,
): Tick[] {
    // start at the step's boundary on or before the window's first day
    let date = boundaryOf(unit, count, TimeZone.date(timeZone, scale.start), week);
    const step = periodOf(unit, count);

    // step through the window by calendar units
    const ticks: Tick[] = [];
    for (let at = TimeZone.instant(timeZone, date, "00:00"); at <= scale.end;) {
        if (at >= scale.start) {
            ticks.push({ at, unit });
        }
        date = PlainDate.add(date, step);
        at = TimeZone.instant(timeZone, date, "00:00");
    }

    return ticks;
}

/** Find the boundary of a calendar step on or before a day. */
function boundaryOf(
    unit: "day" | "week" | "month" | "year",
    count: number,
    day: PlainDate,
    week: Week,
): PlainDate {
    // start a day step on the day, a week step on the week's first day
    if (unit === "day") {
        return day;
    } else if (unit === "week") {
        return Week.startOf(week, day);
    }
    // start a month step on a month the count divides, from January
    else if (unit === "month") {
        const month = PlainDate.month(day);

        return PlainDate.of(PlainDate.year(day), month - ((month - 1) % count), 1);
    }

    return PlainDate.of(PlainDate.year(day) - (PlainDate.year(day) % count), 1, 1);
}

/** Read the calendar step of a count of units. */
function periodOf(unit: "day" | "week" | "month" | "year", count: number): Period {
    if (unit === "day") {
        return { days: count };
    } else if (unit === "week") {
        return { weeks: count };
    } else if (unit === "month") {
        return { months: count };
    }

    return { years: count };
}

/** Read the unit a fixed step counts in. */
function unitOf(step: number): TickUnit {
    if (step < 1000) {
        return "millisecond";
    } else if (step < MINUTE) {
        return "second";
    } else if (step < HOUR) {
        return "minute";
    }

    return "hour";
}
