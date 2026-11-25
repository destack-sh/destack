import type { Localization } from "@destack/locale";
import {
    Duration,
    nth,
    type Tick,
    type TickUnit,
    TimeScale,
    TimeZone,
    type Week,
    type WorkCalendar,
} from "@destack/schema";

/** A mark of an axis. */
export interface TimelineTick {
    /** Where it stands. */
    readonly at: number;
    /** What it says, as the scale writes its place by default. */
    readonly label?: string;
    /** Whether it stands for now, such as today. */
    readonly isCurrent?: boolean;
    /** The unit the step between marks counts in, none on a scale of the caller's. */
    readonly unit?: TickUnit;
}

/** A stretch of an axis above its marks, such as a month above its days. */
export interface TimelinePeriod extends TimeScale {
    /** What it says. */
    readonly label: string;
}

/** What an axis shows: a range, on a track of a width in pixels. */
export interface TimelineView {
    /** The range shown. */
    readonly range: TimeScale;
    /** The width of the track in pixels, zero until measured. */
    readonly width: number;
}

/** What an axis counts in whole numbers, such as dates, or the frames of a film. */
export interface TimelineScale {
    /** List the marks of a view, as many as fit their labels. */
    readonly ticks: (view: TimelineView) => readonly TimelineTick[];
    /** Write a place, such as where a drag lands. */
    readonly label: (at: number) => string;
    /** Move a place to the nearest one things land on. */
    readonly snap: (at: number, view: TimelineView) => number;
    /** Read the place a step beside one, after or before it, the next mark by default. */
    readonly step?: (at: number, direction: 1 | -1, view: TimelineView) => number;
    /** Write a stretch, its two places by default. */
    readonly span?: (range: TimeScale, view: TimelineView) => string;
    /** Write one place of a stretch, as it is written alone by default. */
    readonly brief?: (at: number, view: TimelineView) => string;
    /** Write how long a stretch is, left out by default. */
    readonly duration?: (length: number) => string;
    /** List the periods above the marks, none by default. */
    readonly periods?: (view: TimelineView) => readonly TimelinePeriod[];
}

/** How an axis reads: as dates, as offsets from the first start, or by a scale of the caller's. */
export type TimelineAxisMode = "time" | "offset" | TimelineScale;

/** The date and time parts a period above the marks shows: November 2026 above the days. */
const PERIOD_LABELS: Readonly<Record<TickUnit, Intl.DateTimeFormatOptions>> = {
    millisecond: { second: "2-digit" },
    second: { hour: "numeric", minute: "2-digit", second: "2-digit" },
    minute: { hour: "numeric", minute: "2-digit" },
    hour: { weekday: "short", day: "numeric", month: "short", hour: "numeric" },
    day: { weekday: "short", day: "numeric", month: "short" },
    week: { day: "numeric", month: "short" },
    month: { month: "long", year: "numeric" },
    year: { year: "numeric" },
};

/** The date and time parts a mark shows under its period, such as 18 under November. */
const MARK_LABELS: Readonly<Record<TickUnit, Intl.DateTimeFormatOptions>> = {
    millisecond: { second: "2-digit", fractionalSecondDigits: 3 },
    second: { second: "2-digit" },
    minute: { hour: "numeric", minute: "2-digit" },
    hour: { hour: "numeric" },
    day: { day: "numeric" },
    week: { day: "numeric" },
    month: { month: "short" },
    year: { year: "numeric" },
};

/** The units of marks within a day, which say their day at midnight. */
export const DAY_UNITS: ReadonlySet<TickUnit> = new Set([
    "hour",
    "minute",
    "second",
    "millisecond",
]);

/** The units a duration is written in, largest first, with their milliseconds. */
const DURATION_UNITS: readonly (readonly [string, number])[] = [
    ["day", Duration.milliseconds({ days: 1 })],
    ["hour", Duration.milliseconds({ hours: 1 })],
    ["minute", Duration.milliseconds({ minutes: 1 })],
    ["second", Duration.milliseconds({ seconds: 1 })],
    ["millisecond", 1],
];

/** The pixels between marks with short labels at least, such as 18 under November. */
const MARK_SPACING = 32;

/** The pixels between marks of each unit at least, by the width of their short labels. */
const MARK_SPACINGS: Readonly<Record<TickUnit, number>> = {
    millisecond: 64,
    second: 64,
    minute: 64,
    hour: 64,
    day: MARK_SPACING,
    week: MARK_SPACING,
    month: 40,
    year: 48,
};

/** The pixels between marks with long labels at least, such as 400ms or 2026. */
const TICK_SPACING = 80;

/** Write a duration in its largest fitting unit, such as 850 ms, 2.4 s or 3.5 h. */
function writeDuration(length: number, locale: Localization, scale = length): string {
    // pick the largest unit the scale fills, or the smallest
    const unit =
        DURATION_UNITS.find(([, each]) => Math.abs(scale) >= each) ?? nth(DURATION_UNITS, -1);

    return locale.number(length / unit[1], {
        style: "unit",
        unit: unit[0],
        unitDisplay: "narrow",
        maximumFractionDigits: 1,
    });
}

/** Count the marks a view takes, as many as fit at their labels' width. */
function markCountOf({ range, width }: TimelineView, week: Week): number {
    // try the tightest spacing, then space the unit it gives by its labels' width
    const count = (spacing: number) => Math.max(1, Math.floor(width / spacing));
    const [first] = TimeScale.ticks(range, count(MARK_SPACING), "UTC", week);

    return count(first === undefined ? MARK_SPACING : MARK_SPACINGS[first.unit]);
}

/** Count the marks with long labels a view takes. */
function wideCountOf({ width }: TimelineView): number {
    return Math.max(1, Math.floor(width / TICK_SPACING));
}

/** The time since an origin, such as of a trace, marked on round numbers of milliseconds. */
export class ElapsedScale implements TimelineScale {
    /** The locale it is written in. */
    readonly #locale: Localization;
    /** Read where it counts from. */
    readonly #origin: () => number;

    /** Count from an origin, zero by default. */
    constructor(locale: Localization, origin: () => number = () => 0) {
        this.#locale = locale;
        this.#origin = origin;
    }

    /** List marks on round numbers from the origin. */
    ticks(view: TimelineView): readonly TimelineTick[] {
        // mark from zero, and stand the marks at the origin
        const origin = this.#origin();
        const { range } = this.#fromOrigin(view);
        const ticks = TimeScale.ticks(range, wideCountOf(view), "UTC", this.#locale.week);

        return ticks.map((tick) => ({
            unit: tick.unit,
            at: tick.at + origin,
            label: this.#mark(tick, range.end - range.start),
        }));
    }

    /** Write a mark in its unit, such as 120 ms or 1.5 s. */
    #mark(tick: Tick, length: number): string {
        return tick.unit === "millisecond"
            ? this.#locale.number(tick.at, {
                  style: "unit",
                  unit: "millisecond",
                  unitDisplay: "narrow",
              })
            : writeDuration(tick.at, this.#locale, tick.at === 0 ? length : tick.at);
    }

    /** Write the time since the origin. */
    label(at: number): string {
        return writeDuration(at - this.#origin(), this.#locale);
    }

    /** Read a view from the origin. */
    #fromOrigin({ range, width }: TimelineView): TimelineView {
        const origin = this.#origin();

        return { range: { start: range.start - origin, end: range.end - origin }, width };
    }

    /** Move a time to the nearest mark. */
    snap(at: number, view: TimelineView): number {
        // count from the origin, as the marks do
        const { week } = this.#locale;
        const origin = this.#origin();
        const { range } = this.#fromOrigin(view);

        return origin + TimeScale.snap(range, wideCountOf(view), at - origin, "UTC", week);
    }

    /** Read the mark beside a time. */
    step(at: number, direction: 1 | -1, view: TimelineView): number {
        // count from the origin, as the marks do
        const { week } = this.#locale;
        const origin = this.#origin();
        const { range } = this.#fromOrigin(view);
        const count = wideCountOf(view);

        return origin + TimeScale.markBeside(range, count, at - origin, direction, "UTC", week);
    }

    /** Write how long a stretch takes. */
    duration(length: number): string {
        return writeDuration(length, this.#locale);
    }
}

/** What a scale of dates reads besides its locale. */
export interface DateScaleOptions {
    /** Read the IANA time zone it marks local times in. */
    readonly timeZone: () => string;
    /** Read the calendar its track counts, which leaves out hidden days. */
    readonly calendar: () => WorkCalendar;
    /** Read the current time, unknown until the view is mounted. */
    readonly now: () => number | undefined;
    /** Read the unit things land on, the marks while undefined. */
    readonly unit: () => TickUnit | undefined;
}

/** The dates and times of a time zone, marked under their periods. */
export class DateScale implements TimelineScale {
    /** The locale it is written in. */
    readonly #locale: Localization;
    /** What it reads besides. */
    readonly #options: DateScaleOptions;

    /** Mark dates in a locale. */
    constructor(locale: Localization, options: DateScaleOptions) {
        this.#locale = locale;
        this.#options = options;
    }

    /** Read a view as wide as its range would be with the hidden days. */
    #whole(view: TimelineView): TimelineView {
        const { start, end } = view.range;
        const length = this.#options.calendar().between(start, end);

        return { range: view.range, width: (view.width * (end - start)) / length };
    }

    /** Read the marks of a view before their labels. */
    #marks(view: TimelineView): readonly Tick[] {
        // mark times densely with short labels under periods, else sparsely with long ones
        const zone = this.#options.timeZone();
        const { week } = this.#locale;
        const dense = TimeScale.ticks(view.range, markCountOf(this.#whole(view), week), zone, week);
        const [first] = dense;
        const ticks =
            first !== undefined && TimeScale.periodOf(first.unit) !== undefined
                ? dense
                : TimeScale.ticks(view.range, wideCountOf(view), zone, week);

        // leave out a mark the next one draws over, as days hidden before a Monday
        const calendar = this.#options.calendar();

        return ticks.filter((tick, index) => {
            const next = ticks[index + 1];

            return next === undefined || calendar.between(tick.at, next.at) > 0;
        });
    }

    /** List the marks of a view, a day's mark current where it is today. */
    ticks(view: TimelineView): readonly TimelineTick[] {
        // find today, and write each mark at its unit's precision
        const timeZone = this.#options.timeZone();
        const now = this.#options.now();
        const today = now === undefined ? undefined : TimeZone.date(timeZone, now);

        return this.#marks(view).map((tick) => ({
            unit: tick.unit,
            at: tick.at,
            label: this.#locale.date(tick.at, { ...MARK_LABELS[tick.unit], timeZone }),
            isCurrent: tick.unit === "day" && TimeZone.date(timeZone, tick.at) === today,
        }));
    }

    /** Write a time with its date. */
    label(at: number): string {
        return this.#locale.date(at, {
            dateStyle: "medium",
            timeStyle: "short",
            timeZone: this.#options.timeZone(),
        });
    }

    /** Move a time to the nearest mark or boundary of the unit things land on. */
    snap(at: number, view: TimelineView): number {
        // take the nearest mark, or the unit's boundary
        const { week } = this.#locale;
        const zone = this.#options.timeZone();
        const unit = this.#options.unit();

        return unit === undefined
            ? TimeScale.snap(view.range, markCountOf(this.#whole(view), week), at, zone, week)
            : TimeScale.round(unit, at, zone, week);
    }

    /** Read the mark or the boundary of the unit beside a time. */
    step(at: number, direction: 1 | -1, view: TimelineView): number {
        // take the next mark, or the unit's next boundary
        const { week } = this.#locale;
        const zone = this.#options.timeZone();
        const unit = this.#options.unit();

        return unit === undefined
            ? TimeScale.markBeside(
                  view.range,
                  markCountOf(this.#whole(view), week),
                  at,
                  direction,
                  zone,
                  week,
              )
            : TimeScale.boundaryBeside(unit, at, direction, zone, week);
    }

    /** Read the fields of a date at a view's precision. */
    #fields(view: TimelineView): Intl.DateTimeFormatOptions {
        return {
            month: "short",
            day: "numeric",
            ...(this.#isWithinDay(view) ? { hour: "numeric", minute: "2-digit" } : {}),
            timeZone: this.#options.timeZone(),
        };
    }

    /** Report whether a view marks the hours of days. */
    #isWithinDay(view: TimelineView): boolean {
        const [first] = view.width === 0 ? [] : this.#marks(view);

        return first !== undefined && DAY_UNITS.has(first.unit);
    }

    /** Write a range by its last covered day, its end being the first instant after it. */
    span({ start, end }: TimeScale, view: TimelineView): string {
        const last = this.#isWithinDay(view) ? end : end - 1;

        return this.#locale.dateRange(start, last, this.#fields(view));
    }

    /** Write a date at a view's precision, such as Nov 6. */
    brief(at: number, view: TimelineView): string {
        return this.#locale.date(at, this.#fields(view));
    }

    /** Write how long a stretch takes. */
    duration(length: number): string {
        return writeDuration(length, this.#locale);
    }

    /** List the periods above the marks, the first and last cut to the view. */
    periods(view: TimelineView): readonly TimelinePeriod[] {
        // group the marks by the unit above theirs, such as days by their month
        const [first] = view.width === 0 ? [] : this.#marks(view);
        const unit = first === undefined ? undefined : TimeScale.periodOf(first.unit);
        if (unit === undefined) {
            return [];
        }
        const timeZone = this.#options.timeZone();

        return TimeScale.periods(view.range, unit, timeZone, this.#locale.week).map((period) => ({
            ...period,
            label: this.#locale.date(period.start, { ...PERIOD_LABELS[unit], timeZone }),
        }));
    }
}

/** Report whether an axis reads as dates, as it does by default. */
export function isDated(axis: TimelineAxisMode | undefined): boolean {
    return axis === undefined || axis === "time";
}
