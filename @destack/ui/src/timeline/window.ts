import { t } from "@destack/locale";
import { Duration, type ScheduleEdge, TimeScale, WorkCalendar, WorkWeek } from "@destack/schema";
import {
    type Accessor,
    createMemo,
    createSignal,
    untrack,
    type useEnvironment,
    type useLocale,
} from "@destack/view";
import { createElementSize } from "@destack/view/primitives/resize-observer";
import type { TimelineOptions } from "./control.ts";
import {
    DateScale,
    DAY_UNITS,
    ElapsedScale,
    isDated,
    type TimelinePeriod,
    type TimelineScale,
    type TimelineTick,
    type TimelineView,
} from "./scale.ts";
import type { TimelineItem, TimelineTree } from "./tree.ts";

/** The zoom levels of a timeline: a week, a month or a quarter per column. */
export const TIMELINE_ZOOM_LEVELS = ["week", "month", "quarter"] as const;

/** A zoom level of a timeline. */
export type TimelineZoomLevel = (typeof TIMELINE_ZOOM_LEVELS)[number];

/** The milliseconds of a day of UTC. */
export const DAY = Duration.milliseconds({ days: 1 });

/** The shortest window, ten of what the axis counts, such as frames. */
const LEAST_LENGTH = 10;

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

/** The time a timeline shows, how it is marked, and how instants map onto its track. */
export class TimelineWindow {
    /** The span shown, moved on to working time where hidden days would cover it whole. */
    readonly range: Accessor<TimeScale>;
    /** The track's width in pixels, none until measured. */
    readonly width: Accessor<number | undefined>;
    /** The element whose width is the track's, none until mounted. */
    readonly track: Accessor<HTMLElement | undefined>;
    /** The marks of the axis, none until the track has a width. */
    readonly ticks: Accessor<readonly TimelineTick[]>;
    /** The calendar the track counts: the working days while hidden, else every millisecond. */
    readonly calendar: Accessor<WorkCalendar>;
    /** The non-working days to shade, none while hidden or while marks are wider than days. */
    readonly nonWorking: Accessor<TimeScale[]>;
    /** The periods above the marks, the first and last cut to the window. */
    readonly periods: Accessor<readonly TimelinePeriod[]>;
    /** Set the element whose width is the track's. */
    readonly attachTrack: (element: HTMLElement | undefined) => void;
    /** Show another span and tell the owner. */
    readonly #show: (range: TimeScale) => void;
    /** The working time of the owner's week or the locale's, which the grid shades. */
    readonly #workDays: Accessor<WorkCalendar>;
    /** The window's length in the track's time. */
    readonly #length: Accessor<number>;
    /** What the axis counts in. */
    readonly scale: Accessor<TimelineScale>;
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
        // show a trace's items as they grow until the person zooms, a plan's as it opened
        this.#options = options;
        this.#tree = tree;
        this.#locale = locale;
        this.#environment = environment;
        this.#ticking = ticking;
        const [chosen, choose] = createSignal<TimeScale | undefined>(
            untrack(
                () => options.defaultWindow ?? (isDated(options.axis) ? tree.span() : undefined),
            ),
            { ownedWrite: true },
        );
        const shown = (): TimeScale => options.window ?? chosen() ?? tree.span();
        this.#show = (range) => {
            // keep the span the person chose unless the owner controls it
            if (options.window === undefined) {
                choose(range);
            }
            options.onWindowChange?.(range);
        };

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

        // count in dates, in offsets from the first start, or by the caller's scale
        const dates = new DateScale(locale, {
            timeZone: () => this.timeZone(),
            calendar: () => this.calendar(),
            now: () => this.now(),
            unit: () => (options.snap === false ? undefined : options.snap),
        });
        const elapsed = new ElapsedScale(locale, () => tree.origin());
        this.scale = () =>
            typeof options.axis === "object"
                ? options.axis
                : options.axis === "offset"
                  ? elapsed
                  : dates;

        // mark the window once per change, nothing until the track has a width
        this.ticks = createMemo(() => {
            const view = this.view();

            return view.width === 0 ? [] : this.scale().ticks(view);
        });
        this.periods = createMemo(() => this.scale().periods?.(this.view()) ?? []);

        // shade the weekends while they show and the marks are days
        this.nonWorking = createMemo(() => {
            // shade marks of days or finer over a short window
            const [first] = this.ticks();
            const range = this.range();
            const isDaily =
                first?.unit !== undefined && (first.unit === "day" || DAY_UNITS.has(first.unit));
            const isShaded =
                isDated(options.axis) &&
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
        const length = Math.max(LEAST_LENGTH, Math.round(this.#length() / factor));
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

    /** Read what the axis shows. */
    view(): TimelineView {
        return { range: this.range(), width: this.width() ?? 0 };
    }

    /** Snap an edge to where things land, or to nothing, off hidden days. */
    snap(at: number, edge: ScheduleEdge): number {
        // leave the place where the owner snaps to nothing
        const calendar = this.calendar();
        if (this.#options.snap === false) {
            return calendar.align(at, edge);
        }

        // take where things land, then the place beside it if nearer on the track
        const nearest = this.scale().snap(at, this.view());
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

    /** Write a place for a person, such as its date and time. */
    instantLabel(at: number): string {
        return this.scale().label(at);
    }

    /** Read a level's unit in the track's time around an instant, such as five working days. */
    #unitLength(level: TimelineZoomLevel, around: number): number {
        const half = LEVEL_UNITS[level] / 2;

        return this.calendar().between(around - half, around + half);
    }

    /** Read the place a step beside one, after or before it, the next mark by default. */
    #beside(at: number, direction: 1 | -1): number {
        // take the scale's own step
        const scale = this.scale();
        const view = this.view();
        if (scale.step !== undefined) {
            return scale.step(at, direction, view);
        }

        // look for a mark as far as the window is long
        const length = view.range.end - view.range.start;
        const around = {
            range: { start: at - length, end: at + length },
            width: 2 * view.width,
        };
        const marks = scale.ticks(around).map((tick) => tick.at);
        const next =
            direction === 1 ? marks.find((mark) => mark > at) : marks.findLast((mark) => mark < at);

        return next ?? at;
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

    /** Write a mark's label. */
    tickLabel(tick: TimelineTick): string {
        return tick.label ?? this.scale().label(tick.at);
    }

    /** Write the stretch an item spans at the marks' precision, such as Nov 6 – 12. */
    rangeLabel({ start, end }: Pick<TimelineItem, "start" | "end">): string {
        // write a place as the scale writes it in a stretch
        const scale = this.scale();
        const view = this.view();
        const write = (at: number): string => scale.brief?.(at, view) ?? scale.label(at);
        if (start !== undefined && end !== undefined) {
            return start === end
                ? write(start)
                : (scale.span?.({ start, end }, view) ?? `${write(start)} – ${write(end)}`);
        }
        // write the one place of an open item
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

    /** Write how long a stretch is, none on a scale that does not say. */
    durationLabel(length: number): string | undefined {
        return this.scale().duration?.(length);
    }
}
