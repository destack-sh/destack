import { Duration, schema } from "@destack/schema";
import { Placeholder } from "../sql/index.ts";

/** Microseconds per millisecond. */
const MICROSECONDS_PER_MILLISECOND = 1000;

/** The prefix of the placeholders that read the time, followed by their scale, offset and unit. */
const NOW = "now";

/** A placeholder's name as a time: its scale, its offset and the unit it rounds down to. */
const NAME = /^now(\d+)_(-?\d+)(?:_(day|week|month))?$/u;

/** The calendar units a time rounds down to. */
const UNIT = schema.enum(["day", "week", "month"]);

/** The milliseconds of a day. */
const DAY_MILLISECONDS = 86_400_000;

/** The calendar units a time rounds down to, as Elasticsearch's date math rounds `now/d`. */
export type NowUnit = schema.Infer<typeof UNIT>;

/** The moment relative times read from, and the time zone their days fall in. */
export interface Moment {
    /** The time, in milliseconds. */
    readonly now: number;
    /** The time zone days, weeks and months start in, such as Europe/Vienna. */
    readonly timeZone: string;
}

/** The time a condition compares a field to: now, at an offset, rounded down to a unit. */
export class Now extends Placeholder {
    /** The offset from the time, in milliseconds, its whole days by the calendar after rounding. */
    readonly offset: number;
    /** The compared field's units per millisecond: 1 for milliseconds, 1,000 for microseconds. */
    readonly scale: number;
    /** The unit the time rounds down to before the offset, absent to read it exactly. */
    readonly unit: NowUnit | undefined;

    /** Read the time at an offset, rounded down to a unit, in a field's units. */
    constructor(offset = 0, scale = 1, unit?: NowUnit) {
        // name the placeholder by the scale, the offset and the unit
        super(`${NOW}${String(scale)}_${String(offset)}${unit === undefined ? "" : `_${unit}`}`);
        this.offset = offset;
        this.scale = scale;
        this.unit = unit;
    }

    /** Read a placeholder's name as a time, absent for any other placeholder. */
    static parse(name: string): Now | undefined {
        // read the scale, the offset and the unit from the name, none from any other
        const match = NAME.exec(name);
        if (match === null) {
            return undefined;
        }

        // build the time from its scale, offset and unit
        const [, scale, offset, unit] = match;

        return new Now(
            Number(offset),
            Number(scale),
            unit === undefined ? undefined : UNIT.parse(unit),
        );
    }

    /** Read the time a duration earlier. */
    minus(duration: Duration): Now {
        return new Now(this.offset - Duration.milliseconds(duration), this.scale, this.unit);
    }

    /** Read the time a duration later. */
    plus(duration: Duration): Now {
        return new Now(this.offset + Duration.milliseconds(duration), this.scale, this.unit);
    }

    /** Read the time rounded down to the start of its day, ISO week or month. */
    start(unit: NowUnit): Now {
        return new Now(this.offset, this.scale, unit);
    }

    /** Read the time in microseconds, for fields such as event times. */
    microseconds(): Now {
        return new Now(this.offset, MICROSECONDS_PER_MILLISECOND, this.unit);
    }

    /** Bind the time at a moment in the field's units, rounded in the moment's time zone. */
    at(moment: Moment): number {
        // read it exactly, at its offset
        if (this.unit === undefined) {
            return (moment.now + this.offset) * this.scale;
        }

        // step whole days by the calendar, as a day has 23 to 25 hours, then the rest of the offset
        const days = Math.trunc(this.offset / DAY_MILLISECONDS);
        const rest = this.offset - days * DAY_MILLISECONDS;

        return (startOf(moment, this.unit, days) + rest) * this.scale;
    }

    /** Read the time at which a field's value comes due, inclusive or after it. */
    due(value: number, isInclusive: boolean): number {
        return isInclusive
            ? Math.ceil(value / this.scale) - this.offset
            : Math.floor(value / this.scale) - this.offset + 1;
    }
}

/** The time, which conditions compare time fields to. */
export const now = new Now();

/** Read the start of a moment's day, ISO week or month in its time zone, some days from there. */
function startOf(moment: Moment, unit: NowUnit, days: number): number {
    // read the moment's local date and ISO weekday
    const parts = new Intl.DateTimeFormat("en-US", {
        timeZone: moment.timeZone,
        year: "numeric",
        month: "numeric",
        day: "numeric",
    }).formatToParts(moment.now);
    const part = (type: Intl.DateTimeFormatPartTypes) =>
        Number(parts.find((each) => each.type === type)?.value);
    const date = Date.UTC(part("year"), part("month") - 1, part("day"));
    const weekday = new Date(date).getUTCDay() || 7;

    // step back to the unit's first date and by the days, then to that date's local midnight
    const start =
        unit === "day"
            ? date
            : unit === "week"
              ? date - (weekday - 1) * DAY_MILLISECONDS
              : Date.UTC(part("year"), part("month") - 1, 1);
    const first = start + days * DAY_MILLISECONDS;

    return first - offsetOf(first - offsetOf(first, moment.timeZone), moment.timeZone);
}

/** Read a time zone's offset from UTC at an instant, in milliseconds. */
function offsetOf(instant: number, timeZone: string): number {
    // read the zone's wall clock at the instant as if it were UTC
    const parts = new Intl.DateTimeFormat("en-US", {
        timeZone,
        hourCycle: "h23",
        year: "numeric",
        month: "numeric",
        day: "numeric",
        hour: "numeric",
        minute: "numeric",
        second: "numeric",
    }).formatToParts(instant);
    const part = (type: Intl.DateTimeFormatPartTypes) =>
        Number(parts.find((each) => each.type === type)?.value);
    const wall = Date.UTC(
        part("year"),
        part("month") - 1,
        part("day"),
        part("hour"),
        part("minute"),
        part("second"),
    );

    return wall - Math.floor(instant / 1000) * 1000;
}
