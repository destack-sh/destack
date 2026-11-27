import * as schema from "../validate/index.ts";

/** The units of a duration, largest first, as Temporal.Duration names them. */
export const DURATION_UNITS = [
    "years",
    "months",
    "weeks",
    "days",
    "hours",
    "minutes",
    "seconds",
    "milliseconds",
    "microseconds",
    "nanoseconds",
] as const;

/** A unit of a duration. */
export const DurationUnit = schema.enum(DURATION_UNITS);
/** A unit of a duration. */
export type DurationUnit = schema.Infer<typeof DurationUnit>;

/** The milliseconds in each unit of a fixed length, an ISO week seven days. */
const UNIT_MILLISECONDS = {
    weeks: 604_800_000,
    days: 86_400_000,
    hours: 3_600_000,
    minutes: 60_000,
    seconds: 1000,
    milliseconds: 1,
    microseconds: 0.001,
    nanoseconds: 0.000_001,
} as const;

/** The months of a year, by which a fraction of a year carries into months. */
const MONTHS_PER_YEAR = 12;

/** The units of a fixed length, largest first. */
const FIXED_UNITS = [
    "weeks",
    "days",
    "hours",
    "minutes",
    "seconds",
    "milliseconds",
    "microseconds",
    "nanoseconds",
] as const;

/** The nanoseconds in each unit of a fixed length, an ISO week seven days. */
const UNIT_NANOSECONDS = {
    weeks: 604_800_000_000_000,
    days: 86_400_000_000_000,
    hours: 3_600_000_000_000,
    minutes: 60_000_000_000,
    seconds: 1_000_000_000,
    milliseconds: 1_000_000,
    microseconds: 1000,
    nanoseconds: 1,
} as const;

/** The units whose length depends on the date they start at, as Temporal needs a date for. */
const CALENDAR_UNITS = ["years", "months"] as const;

/** The letters of the ISO 8601 form before its time, after P. */
const DATE_LETTERS = { years: "Y", months: "M", weeks: "W", days: "D" } as const;

/** A duration's ISO 8601 form, as Temporal writes and reads it, to nine digits of a second. */
const ISO =
    /^([+-])?P(?:(\d+)Y)?(?:(\d+)M)?(?:(\d+)W)?(?:(\d+)D)?(?:T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)(?:\.(\d{1,9}))?S)?)?$/u;

/** The digits of the sub-second units within a second's fraction: milli, micro, nano. */
const FRACTION_DIGITS = 9;

/** A duration's units, each a whole number of one sign, as Temporal.Duration holds them. */
function units(sign: 1 | -1) {
    const unit = () => {
        const whole = schema.number().int();

        return (sign === 1 ? whole.min(0) : whole.max(0)).exactOptional();
    };

    return schema.object({
        /** Years, of the calendar. */
        years: unit(),
        /** Months, of the calendar. */
        months: unit(),
        /** Weeks of seven days. */
        weeks: unit(),
        /** Days of 24 hours. */
        days: unit(),
        /** Hours. */
        hours: unit(),
        /** Minutes. */
        minutes: unit(),
        /** Seconds. */
        seconds: unit(),
        /** Milliseconds. */
        milliseconds: unit(),
        /** Microseconds. */
        microseconds: unit(),
        /** Nanoseconds. */
        nanoseconds: unit(),
    });
}

/** The schema of a duration: its units, all at or above zero, or all at or below. */
const durationSchema = schema
    .union([units(1), units(-1)])
    .meta({ id: "schema.duration", title: "Duration" });

/** A span of time, as Temporal.Duration: whole units of one sign, the calendar's among them. */
export type Duration = Readonly<schema.Infer<typeof durationSchema>>;

/** A span of time, as Temporal.Duration: whole units of one sign, the calendar's among them. */
export const Duration = Object.assign(durationSchema, {
    /** A duration forward in time, every unit at or above zero, such as a wait or a timeout. */
    forward: units(1),

    /** Write an amount of a unit as whole units, such as 1.5 hours as 1 hour and 30 minutes. */
    of(amount: number, unit: DurationUnit): Duration {
        // write a whole amount as it is, and carry a year's fraction into months
        const whole = Math.trunc(amount);
        if (amount === whole) {
            return durationSchema.parse(whole === 0 ? {} : { [unit]: whole });
        } else if (unit === "years") {
            const months = Duration.of((amount - whole) * MONTHS_PER_YEAR, "months");

            return durationSchema.parse({ ...(whole === 0 ? {} : { years: whole }), ...months });
        } else if (unit === "months") {
            throw new RangeError("duration has a fraction of a month, whose length needs a date");
        }

        // split a fixed unit's amount from it down to whole nanoseconds
        let rest = Math.round(amount * UNIT_NANOSECONDS[unit]);
        const parts: Partial<Record<DurationUnit, number>> = {};
        for (const each of FIXED_UNITS.slice(FIXED_UNITS.indexOf(unit))) {
            const part = Math.trunc(rest / UNIT_NANOSECONDS[each]);
            rest -= part * UNIT_NANOSECONDS[each];
            if (part !== 0) {
                parts[each] = part;
            }
        }

        return durationSchema.parse(parts);
    },

    /** Read a duration's sign: 1 forward, -1 back, 0 for none. */
    sign(duration: Duration): -1 | 0 | 1 {
        const values = DURATION_UNITS.map((unit) => duration[unit] ?? 0);

        return values.some((value) => value < 0) ? -1 : values.some((value) => value > 0) ? 1 : 0;
    },

    /** Measure a duration in milliseconds, refusing years and months, whose length needs a date. */
    milliseconds(duration: Duration): number {
        // refuse the calendar's units, as Temporal's total does without a date
        const calendar = CALENDAR_UNITS.find((unit) => (duration[unit] ?? 0) !== 0);
        if (calendar !== undefined) {
            throw new RangeError(`duration has '${calendar}', whose length needs a date`);
        }

        // add up the units of fixed length
        return Object.entries(UNIT_MILLISECONDS).reduce(
            (total, [unit, length]) => total + (duration[DurationUnit.parse(unit)] ?? 0) * length,
            0,
        );
    },

    /** Measure a duration in seconds, refusing years and months, whose length needs a date. */
    seconds(duration: Duration): number {
        return Duration.milliseconds(duration) / UNIT_MILLISECONDS.seconds;
    },

    /** Require a duration of fixed-length units at or above zero, at least one of them. */
    require(duration: unknown, name: string): Duration {
        // refuse anything but a forward duration with a unit, and the calendar's units
        const parsed = durationSchema.safeParse(duration);
        const isFixed =
            parsed.success &&
            Object.keys(parsed.data).length > 0 &&
            Duration.sign(parsed.data) >= 0 &&
            CALENDAR_UNITS.every((unit) => parsed.data[unit] === undefined);
        if (!parsed.success || !isFixed) {
            throw new TypeError(`'${name}' is not a duration: ${JSON.stringify(duration)}`);
        }

        return parsed.data;
    },

    /** Write a duration in ISO 8601, as Temporal.Duration's toString, such as `P1DT2H30M`. */
    iso(duration: Duration): string {
        // write the date's units, then the time's, the sub-second ones as a fraction of seconds
        const sign = Duration.sign(duration) < 0 ? "-" : "";
        const magnitude = (unit: DurationUnit) => Math.abs(duration[unit] ?? 0);
        const date = Object.entries(DATE_LETTERS)
            .map(([unit, letter]) => [magnitude(DurationUnit.parse(unit)), letter] as const)
            .filter(([value]) => value !== 0)
            .map(([value, letter]) => `${String(value)}${letter}`)
            .join("");
        const nanoseconds =
            magnitude("milliseconds") * 1_000_000 +
            magnitude("microseconds") * 1000 +
            magnitude("nanoseconds");
        const seconds = magnitude("seconds") + Math.floor(nanoseconds / 1e9);
        const fraction = String(nanoseconds % 1e9)
            .padStart(FRACTION_DIGITS, "0")
            .replace(/0+$/u, "");

        // write the time's units, and zero seconds for a duration of nothing
        const time = [
            magnitude("hours") === 0 ? "" : `${String(magnitude("hours"))}H`,
            magnitude("minutes") === 0 ? "" : `${String(magnitude("minutes"))}M`,
            seconds === 0 && fraction === ""
                ? ""
                : `${String(seconds)}${fraction === "" ? "" : `.${fraction}`}S`,
        ].join("");
        const written = `${date}${time === "" ? "" : `T${time}`}`;

        return written === "" ? "PT0S" : `${sign}P${written}`;
    },

    /** Read a duration from ISO 8601, as Temporal.Duration.from does, undefined for other text. */
    read(text: string): Duration | undefined {
        // read each unit's digits, the seconds' fraction as milli, micro and nanoseconds
        const match = ISO.exec(text.trim());
        if (match === null || text.trim().endsWith("T") || /^[+-]?P$/u.test(text.trim())) {
            return undefined;
        }
        const [, sign, years, months, weeks, days, hours, minutes, seconds, fraction] = match;
        const nanoseconds = Number((fraction ?? "").padEnd(FRACTION_DIGITS, "0"));
        const read = {
            years,
            months,
            weeks,
            days,
            hours,
            minutes,
            seconds,
            milliseconds:
                fraction === undefined ? undefined : String(Math.floor(nanoseconds / 1e6)),
            microseconds:
                fraction === undefined ? undefined : String(Math.floor(nanoseconds / 1e3) % 1000),
            nanoseconds: fraction === undefined ? undefined : String(nanoseconds % 1000),
        };

        // take the units it names, each of the duration's sign
        const factor = sign === "-" ? -1 : 1;
        const entries = Object.entries(read)
            .filter(([, value]) => value !== undefined && Number(value) !== 0)
            .map(([unit, value]) => [unit, factor * Number(value)] as const);

        return durationSchema.parse(Object.fromEntries(entries));
    },
});
