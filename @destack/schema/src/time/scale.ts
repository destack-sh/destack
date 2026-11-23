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

/** The milliseconds of each unit, exact below a day, a calendar unit's at its average. */
const UNIT_LENGTHS = {
    millisecond: 1,
    second: 1000,
    minute: MINUTE,
    hour: HOUR,
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

/** The unit a two-tier axis groups each unit's marks under. */
const PERIOD_UNITS: Readonly<Record<TickUnit, TickUnit | undefined>> = {
    millisecond: "second",
    second: "minute",
    minute: "hour",
    hour: "day",
    day: "month",
    week: "month",
    month: "year",
    year: undefined,
};

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

    /** Read the unit a two-tier axis groups a unit's marks under, none past years. */
    periodOf(unit: TickUnit): TickUnit | undefined {
        return PERIOD_UNITS[unit];
    },

    /** List the periods of a unit that cover a window in a zone, the first and last cut to it. */
    periods(scale: TimeScale, unit: TickUnit, timeZone: TimeZone, week: Week): TimeScale[] {
        // mark the unit's boundaries from the one on or before the window's start
        const boundaries = isCalendarUnit(unit)
            ? calendarTicks(
                  {
                      start: TimeZone.instant(
                          timeZone,
                          boundaryOf(unit, 1, TimeZone.date(timeZone, scale.start), week),
                          "00:00",
                      ),
                      end: scale.end,
                  },
                  unit,
                  1,
                  timeZone,
                  week,
              )
            : fixedTicks(
                  { start: floorOf(scale.start, UNIT_LENGTHS[unit], timeZone), end: scale.end },
                  UNIT_LENGTHS[unit],
                  timeZone,
              );

        // join each boundary before the window's end to the next, cutting the ends to the window
        const inside = boundaries.filter((tick, index) => index === 0 || tick.at < scale.end);

        return inside.map((tick, index) => ({
            start: Math.max(tick.at, scale.start),
            end: Math.min(inside[index + 1]?.at ?? scale.end, scale.end),
        }));
    },

    /** Read the shortest step for a count of ticks. */
    step(scale: TimeScale, count: number): number {
        return (scale.end - scale.start) / Math.max(count, 1);
    },

    /** Read the axis's mark beside an instant: after it, or before it, on the axis's own step. */
    markBeside(
        scale: TimeScale,
        count: number,
        at: number,
        direction: 1 | -1,
        timeZone: TimeZone,
        week: Week,
    ): number {
        // mark four steps around the instant, which picks the same step as the whole axis
        const step = TimeScale.step(scale, count);
        const around = { start: at - 4 * step, end: at + 4 * step };
        const marks = TimeScale.ticks(around, 8, timeZone, week).map((tick) => tick.at);
        const next =
            direction === 1 ? marks.find((mark) => mark > at) : marks.findLast((mark) => mark < at);
        if (next === undefined) {
            throw new RangeError(`time scale has no mark beside '${String(at)}'`);
        }

        return next;
    },

    /** Move an instant to the nearest boundary of a unit in a zone, such as midnight. */
    round(unit: TickUnit, at: number, timeZone: TimeZone, week: Week): number {
        // take the nearest boundary, the later one when two are as near, as Math.round does
        return boundariesOf(at, unit, timeZone, week).reduce((nearest, boundary) =>
            Math.abs(boundary - at) <= Math.abs(nearest - at) ? boundary : nearest,
        );
    },

    /** Read the boundary of a unit in a zone beside an instant: after it, or before it. */
    boundaryBeside(
        unit: TickUnit,
        at: number,
        direction: 1 | -1,
        timeZone: TimeZone,
        week: Week,
    ): number {
        // take the first boundary past the instant in the direction
        const boundaries = boundariesOf(at, unit, timeZone, week);
        const next =
            direction === 1
                ? boundaries.find((boundary) => boundary > at)
                : boundaries.findLast((boundary) => boundary < at);
        if (next === undefined) {
            throw new RangeError(`time scale has no '${unit}' boundary beside '${String(at)}'`);
        }

        return next;
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

/** List the boundaries of a unit in a zone within two of its lengths of an instant. */
function boundariesOf(at: number, unit: TickUnit, timeZone: TimeZone, week: Week): number[] {
    // list the periods around the instant, the first cut to the window and so no boundary
    const length = 2 * UNIT_LENGTHS[unit];
    const periods = TimeScale.periods(
        { start: at - length, end: at + length },
        unit,
        timeZone,
        week,
    );

    return periods.slice(1).map((period) => period.start);
}

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
    const step = stepOf(unit, count);

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
function stepOf(unit: "day" | "week" | "month" | "year", count: number): Period {
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

/** Report whether a unit steps by the calendar rather than by a fixed length. */
function isCalendarUnit(unit: TickUnit): unit is "day" | "week" | "month" | "year" {
    return unit === "day" || unit === "week" || unit === "month" || unit === "year";
}

/** Move an instant back to the boundary of a fixed length, an hour's in local time. */
function floorOf(at: number, length: number, timeZone: TimeZone): number {
    const offset = length >= HOUR ? TimeZone.offset(timeZone, at) : 0;

    return Math.floor((at + offset) / length) * length - offset;
}
