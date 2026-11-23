import { t } from "@destack/locale";
import {
    Duration,
    nth,
    type ScheduleEdge,
    type Tick,
    type TickUnit,
    TimeScale,
    TimeZone,
    type Week,
    WorkCalendar,
    WorkWeek,
} from "@destack/schema";
import {
    type Accessor,
    createControllableSignal,
    createMemo,
    createSignal,
    untrack,
    type useEnvironment,
    type useLocale,
} from "@destack/view";
import { createElementSize } from "@destack/view/primitives/resize-observer";
import type { TimelineOptions } from "./control.ts";
import type { TimelineItem, TimelineTree } from "./tree.ts";

/** The zoom levels of a timeline: a week, a month or a quarter per column. */
export const TIMELINE_ZOOM_LEVELS = ["week", "month", "quarter"] as const;

/** A zoom level of a timeline. */
export type TimelineZoomLevel = (typeof TIMELINE_ZOOM_LEVELS)[number];

/** How a timeline's axis reads: dates and times of a plan, or offsets from a trace's start. */
export type TimelineAxisMode = "time" | "offset";

/** The milliseconds of a day of UTC. */
export const DAY = Duration.milliseconds({ days: 1 });

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
const DAY_UNITS: ReadonlySet<TickUnit> = new Set(["hour", "minute", "second", "millisecond"]);

/** The units a duration is written in, largest first, with their milliseconds. */
const DURATION_UNITS: readonly (readonly [string, number])[] = [
    ["day", Duration.milliseconds({ days: 1 })],
    ["hour", Duration.milliseconds({ hours: 1 })],
    ["minute", Duration.milliseconds({ minutes: 1 })],
    ["second", Duration.milliseconds({ seconds: 1 })],
    ["millisecond", 1],
];

/** The most days a window shades its non-working days over, past which the days are too narrow. */
const SHADED_DAYS = 400;

/** The milliseconds of each zoom level's unit, a month and a quarter at their average length. */
const LEVEL_UNITS: Readonly<Record<TimelineZoomLevel, number>> = {
    week: Duration.milliseconds({ days: 7 }),
    month: Duration.milliseconds({ days: 30.436875 }),
    quarter: Duration.milliseconds({ days: 91.310625 }),
};

/** The share a window's unit width may stray from a level's and still count as that level. */
const LEVEL_TOLERANCE = 0.15;

/** The pixels one unit of a zoom level takes, past the marks' spacing so marks fall on units. */
const UNIT_WIDTH = 120;

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
const TICK_SPACING = 96;

/** The time a timeline shows, how it is marked, and how instants map onto its track. */
export class TimelineWindow {
    /** The span shown, moved on to working time where hidden days would cover it whole. */
    readonly range: Accessor<TimeScale>;
    /** The track's width in pixels, none until measured. */
    readonly width: Accessor<number | undefined>;
    /** The element whose width is the track's, none until mounted. */
    readonly track: Accessor<HTMLElement | undefined>;
    /** The marks of the axis, none until the track has a width. */
    readonly ticks: Accessor<Tick[]>;
    /** The calendar the track counts: the working days while hidden, else every millisecond. */
    readonly calendar: Accessor<WorkCalendar>;
    /** The non-working days to shade, none while hidden or while marks are wider than days. */
    readonly nonWorking: Accessor<TimeScale[]>;
    /** The periods above the marks, the first and last cut to the window. */
    readonly periods: Accessor<TimeScale[]>;
    /** Set the element whose width is the track's. */
    readonly attachTrack: (element: HTMLElement | undefined) => void;
    /** Show another span and tell the owner. */
    readonly #show: (range: TimeScale) => void;
    /** The working time of the owner's week or the locale's, which the grid shades. */
    readonly #workDays: Accessor<WorkCalendar>;
    /** The window's length in the track's time. */
    readonly #length: Accessor<number>;
    /** The count of marks the window takes, none until the track has a width. */
    readonly #countedMarks: Accessor<number | undefined>;
    /** The timeline's options. */
    readonly #options: TimelineOptions;
    /** The items, whose span the window fits and whose origin offsets count from. */
    readonly #tree: TimelineTree;
    /** The locale the axis is marked in. */
    readonly #locale: ReturnType<typeof useLocale>;
    /** The environment's time zone. */
    readonly #environment: ReturnType<typeof useEnvironment>;
    /** The current time, ticking each minute once mounted. */
    readonly #ticking: Accessor<number | undefined>;

    /** Show a timeline's items, or the span its owner controls, in a locale on an environment. */
    constructor(
        options: TimelineOptions,
        tree: TimelineTree,
        locale: ReturnType<typeof useLocale>,
        environment: ReturnType<typeof useEnvironment>,
        ticking: Accessor<number | undefined>,
    ) {
        // show every item until the person zooms, or the span the owner controls
        this.#options = options;
        this.#tree = tree;
        this.#locale = locale;
        this.#environment = environment;
        this.#ticking = ticking;
        const [shown, show] = createControllableSignal<TimeScale>({
            isControlled: () => options.window !== undefined,
            value: () => options.window ?? tree.span(),
            defaultValue: untrack(() => options.defaultWindow ?? tree.span()),
            onChange: (range) => options.onWindowChange?.(range),
        });
        this.#show = show;

        // measure the track
        const [track, setTrack] = createSignal<HTMLElement | undefined>(undefined, {
            ownedWrite: true,
        });
        const measured = createElementSize(track);
        this.track = track;
        this.attachTrack = setTrack;
        this.width = () => measured.width ?? undefined;

        // count working time in the axis's zone where non-working days are hidden
        this.#workDays = createMemo(
            () =>
                new WorkCalendar(
                    this.timeZone(),
                    options.workWeek ?? WorkWeek.of(locale.week.weekend),
                ),
        );
        this.calendar = createMemo(() =>
            options.hideNonWorking === true
                ? this.#workDays()
                : new WorkCalendar("UTC", WorkWeek.of([])),
        );

        // move a window that hidden days cover whole on to the working time after it
        this.range = createMemo(() => {
            // start the window on working time, at the same elapsed length
            const range = shown();
            const start = this.calendar().align(range.start, "start");

            return { start, end: range.end + start - range.start };
        });
        this.#length = createMemo(() => {
            const range = this.range();

            return this.calendar().between(range.start, range.end);
        });

        // mark the window once per change: the count of marks, then the marks and periods
        this.#countedMarks = createMemo(() => {
            // space marks by the elapsed width the track would take, past hidden days
            const width = this.width();
            const range = this.range();
            const elapsed =
                width === undefined
                    ? undefined
                    : (width * (range.end - range.start)) / this.#length();

            return elapsed === undefined
                ? undefined
                : markCountOf(range, elapsed, this.#locale.week);
        });
        this.ticks = createMemo(() => this.#ticksOf());
        this.periods = createMemo(() => {
            const unit = this.periodUnit();

            return unit === undefined
                ? []
                : TimeScale.periods(this.range(), unit, this.timeZone(), this.#locale.week);
        });

        // shade the weekends while they show and the marks are days
        this.nonWorking = createMemo(() => {
            // shade marks of days or finer over a short window
            const [first] = this.ticks();
            const range = this.range();
            const isDaily =
                first !== undefined && (first.unit === "day" || DAY_UNITS.has(first.unit));
            const isShaded =
                options.hideNonWorking !== true &&
                isDaily &&
                range.end - range.start <= SHADED_DAYS * DAY;

            return isShaded ? this.#workDays().nonWorking(range) : [];
        });
    }

    /** Show a span. */
    show(range: TimeScale): void {
        this.#show(range);
    }

    /** Read the time zone the axis marks local times in. */
    timeZone(): string {
        return this.#options.timeZone ?? this.#environment.timeZone;
    }

    /** Read the current time, unknown until the view is mounted. */
    now(): number | undefined {
        return this.#ticking();
    }

    /** Zoom by a factor around the instant at a share of the track, then move by a share. */
    zoom(factor: number, around: number, share = 0): void {
        // keep the instant at the share where it is, in the track's time
        const calendar = this.calendar();
        const pivot = this.instant(around);
        const length = Math.round(this.#length() / factor);
        const start = calendar.add(pivot, Math.round(-around * length));
        this.#showFrom(calendar.add(start, Math.round(share * length)), length);
    }

    /** Move by a share of the window's length, in the track's time. */
    pan(share: number): void {
        const length = this.#length();
        this.#showFrom(this.calendar().add(this.range().start, Math.round(share * length)), length);
    }

    /** Show every item. */
    fit(): void {
        this.show(this.#tree.span());
    }

    /** Zoom to a level, a week, a month or a quarter per column, around the window's middle. */
    zoomTo(level: TimelineZoomLevel): void {
        // fit as many units as the track has columns of the unit's width, around the middle
        const middle = this.instant(0.5);
        const length = Math.round((this.measured() / UNIT_WIDTH) * this.#unitLength(level, middle));
        this.#showFrom(this.calendar().add(middle, -Math.round(length / 2)), length);
    }

    /** Read the level the window shows, none while it shows another zoom. */
    level(): TimelineZoomLevel | undefined {
        // match the width a unit takes now against each level's
        const width = this.width();
        if (width === undefined) {
            return undefined;
        }
        const middle = this.instant(0.5);

        return TIMELINE_ZOOM_LEVELS.find((level) => {
            const unitWidth = (width * this.#unitLength(level, middle)) / this.#length();

            return Math.abs(unitWidth / UNIT_WIDTH - 1) <= LEVEL_TOLERANCE;
        });
    }

    /** Move the window to put an instant in its middle, at the same length. */
    reveal(at: number): void {
        const length = this.#length();
        this.#showFrom(this.calendar().add(at, -Math.round(length / 2)), length);
    }

    /** Read an instant's share of the track, from 0 at its start to 1 at its end. */
    share(at: number): number {
        return this.calendar().between(this.range().start, at) / this.#length();
    }

    /** Read the instant at a share of the track, past hidden days. */
    instant(share: number): number {
        return this.calendar().add(this.range().start, Math.round(share * this.#length()));
    }

    /** Snap an edge to the owner's unit, to nothing, or to the nearest mark, off hidden days. */
    snap(at: number, edge: ScheduleEdge): number {
        // leave the instant where the owner snaps to nothing
        const unit = this.#options.snap;
        const calendar = this.calendar();
        if (unit === false) {
            return calendar.align(at, edge);
        }

        // take the nearest mark or unit boundary in elapsed time, then its neighbour if nearer on the track
        const nearest =
            unit === undefined
                ? TimeScale.snap(
                      this.range(),
                      this.#markCount(),
                      at,
                      this.timeZone(),
                      this.#locale.week,
                  )
                : TimeScale.round(unit, at, this.timeZone(), this.#locale.week);
        const other = this.#beside(nearest, at > nearest ? 1 : -1);
        const isOther =
            Math.abs(calendar.between(at, other)) < Math.abs(calendar.between(at, nearest));

        return calendar.align(isOther ? other : nearest, edge);
    }

    /** Read the step beside an edge, after or before it, past hidden days. */
    step(at: number, direction: 1 | -1, edge: ScheduleEdge): number {
        // step by the owner's unit or by the axis's marks
        const calendar = this.calendar();
        const beside = (from: number) => this.#beside(from, direction);
        const next = beside(at);

        // jump over hidden days a step lands inside, then take the step from their far edge
        if (calendar.between(Math.min(at, next), Math.max(at, next)) === 0) {
            const far = calendar.align(next, direction > 0 ? "start" : "end");

            return calendar.align(beside(far), edge);
        }
        // take the step
        else {
            return calendar.align(next, edge);
        }
    }

    /** Move an item by a duration of the track's time, keeping the working length it has. */
    move(item: TimelineItem, duration: number): TimelineItem {
        // move the start, then end it its working length later
        const calendar = this.calendar();
        if (item.start !== undefined && item.end !== undefined) {
            const start = calendar.align(calendar.add(item.start, duration), "start");

            return {
                ...item,
                start,
                end: calendar.add(start, calendar.between(item.start, item.end)),
            };
        }
        // move the one date an open item has
        else if (item.start !== undefined) {
            return { ...item, start: calendar.align(calendar.add(item.start, duration), "start") };
        } else if (item.end !== undefined) {
            return { ...item, end: calendar.align(calendar.add(item.end, duration), "end") };
        }
        // refuse an item without dates
        else {
            throw new TypeError(`timeline item '${item.key}' moved without dates`);
        }
    }

    /** Write an instant for a person: its date and time in the axis's zone, or its offset. */
    instantLabel(at: number): string {
        return this.#options.axis === "offset"
            ? this.durationLabel(at - this.#tree.origin())
            : this.#locale.date(at, {
                  dateStyle: "medium",
                  timeStyle: "short",
                  timeZone: this.timeZone(),
              });
    }

    /** Read a level's unit in the track's time around an instant, such as five working days. */
    #unitLength(level: TimelineZoomLevel, around: number): number {
        const half = LEVEL_UNITS[level] / 2;

        return this.calendar().between(around - half, around + half);
    }

    /** Read the owner's unit boundary or the axis's mark beside an instant, after or before it. */
    #beside(at: number, direction: 1 | -1): number {
        const unit = this.#options.snap;

        return unit === undefined || unit === false
            ? TimeScale.markBeside(
                  this.range(),
                  this.#markCount(),
                  at,
                  direction,
                  this.timeZone(),
                  this.#locale.week,
              )
            : TimeScale.boundaryBeside(unit, at, direction, this.timeZone(), this.#locale.week);
    }

    /** Show a span from a start, its length counted in the track's time. */
    #showFrom(start: number, length: number): void {
        this.show({ start, end: this.calendar().add(start, length) });
    }

    /** Read the track's width, refusing a track not yet measured, which no drag or snap meets. */
    measured(): number {
        const width = this.width();
        if (width === undefined || width === 0) {
            throw new RangeError("timeline track is invalid: it is not measured yet");
        }

        return width;
    }

    /** Write a mark's label: its local time at its unit's precision, or its offset. */
    tickLabel(tick: Tick): string {
        // write an offset in the mark's unit, such as 120 ms or 1.5 s
        const locale = this.#locale;
        if (this.#options.axis === "offset") {
            const offset = tick.at - this.#tree.origin();

            return tick.unit === "millisecond"
                ? locale.number(offset, {
                      style: "unit",
                      unit: "millisecond",
                      unitDisplay: "narrow",
                  })
                : this.durationLabel(offset);
        }

        // write a time at its unit's precision, its period written above it
        return locale.date(tick.at, { ...MARK_LABELS[tick.unit], timeZone: this.timeZone() });
    }

    /** Read the unit of the periods above the marks: months above days, none on an offset axis. */
    periodUnit(): TickUnit | undefined {
        const [first] = this.ticks();

        return this.#options.axis === "offset" || first === undefined
            ? undefined
            : TimeScale.periodOf(first.unit);
    }

    /** Write a period's label at the precision of the periods' unit, such as November 2026. */
    periodLabel(period: TimeScale): string {
        const unit = this.periodUnit();
        if (unit === undefined) {
            throw new RangeError("timeline period is invalid: the axis has no periods");
        }

        return this.#locale.date(period.start, {
            ...PERIOD_LABELS[unit],
            timeZone: this.timeZone(),
        });
    }

    /** Report whether a mark of days stands on today, which the axis marks as a pill. */
    isToday(tick: Tick): boolean {
        const now = this.now();
        const zone = this.timeZone();

        return (
            tick.unit === "day" &&
            now !== undefined &&
            TimeZone.date(zone, tick.at) === TimeZone.date(zone, now)
        );
    }

    /** Write the dates or times an item spans at the marks' precision, such as Nov 6 – 12. */
    rangeLabel(item: TimelineItem): string {
        // write an offset from the trace's start, or a date at the marks' precision in the zone
        const [first] = this.ticks();
        const isWithinDay = first !== undefined && DAY_UNITS.has(first.unit);
        const fields: Intl.DateTimeFormatOptions = isWithinDay
            ? { month: "short", day: "numeric", hour: "numeric", minute: "2-digit" }
            : { month: "short", day: "numeric" };
        const options = { ...fields, timeZone: this.timeZone() };
        const isOffset = this.#options.axis === "offset";
        const write = (at: number) =>
            isOffset
                ? this.durationLabel(at - this.#tree.origin())
                : this.#locale.date(at, options);
        const { start, end } = item;

        // write a range by its last covered day, its end being the first instant after it
        if (start !== undefined && end !== undefined) {
            const last = isWithinDay || isOffset ? end : end - 1;

            return start === end
                ? write(start)
                : isOffset
                  ? `${write(start)} – ${write(end)}`
                  : this.#locale.dateRange(start, last, options);
        }
        // write the one date of an open item
        else if (start !== undefined) {
            return this.#locale.render(t`From ${write(start)}`);
        } else if (end !== undefined) {
            return this.#locale.render(t`Until ${write(end)}`);
        }
        // write an item without dates
        else {
            return this.#locale.render(t`No dates`);
        }
    }

    /** Write a duration in its largest fitting unit, such as 850 ms, 2.4 s or 3.5 h. */
    durationLabel(duration: number): string {
        // pick the largest unit the duration fills, or the smallest
        const unit =
            DURATION_UNITS.find(([, length]) => Math.abs(duration) >= length) ??
            nth(DURATION_UNITS, -1);

        return this.#locale.number(duration / unit[1], {
            style: "unit",
            unit: unit[0],
            unitDisplay: "narrow",
            maximumFractionDigits: 1,
        });
    }

    /** Read the marks of the window: local times of a plan, offsets from a trace's start. */
    #ticksOf(): Tick[] {
        // mark nothing until the track has a width
        const width = this.width();
        if (width === undefined) {
            return [];
        }

        // mark offsets on round numbers from the origin, with long labels
        const range = this.range();
        const { week } = this.#locale;
        const wide = Math.max(1, Math.floor(width / TICK_SPACING));
        if (this.#options.axis === "offset") {
            const origin = this.#tree.origin();
            const offsets = { start: range.start - origin, end: range.end - origin };

            return TimeScale.ticks(offsets, wide, "UTC", week).map((tick) => ({
                ...tick,
                at: tick.at + origin,
            }));
        }

        // mark times densely with short labels under periods, else sparsely with long ones
        const zone = this.timeZone();
        const dense = TimeScale.ticks(range, this.#markCount(), zone, week);
        const [first] = dense;
        const ticks =
            first !== undefined && TimeScale.periodOf(first.unit) !== undefined
                ? dense
                : TimeScale.ticks(range, wide, zone, week);

        // leave out a mark the next one draws over, as days hidden before a Monday
        const calendar = this.calendar();

        return ticks.filter((tick, index) => {
            const next = ticks[index + 1];

            return next === undefined || calendar.between(tick.at, next.at) > 0;
        });
    }

    /** Count the marks the window takes, as many as fit at their labels' width. */
    #markCount(): number {
        const count = this.#countedMarks();
        if (count === undefined) {
            throw new TypeError("timeline marks read before the track measured");
        }

        return count;
    }
}

/** Count the marks a window takes at a width, as many as fit at their labels' width. */
function markCountOf(range: TimeScale, width: number, week: Week): number {
    // try the tightest spacing, then space the unit it gives by its labels' width
    const count = (spacing: number) => Math.max(1, Math.floor(width / spacing));
    const [first] = TimeScale.ticks(range, count(MARK_SPACING), "UTC", week);

    return count(first === undefined ? MARK_SPACING : MARK_SPACINGS[first.unit]);
}
