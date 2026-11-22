import type { AccentPreset } from "@destack/color";

import {
    nth,
    type DateSpan,
    Duration,
    MonthGrid,
    type Period,
    PlainDate,
    Recurrence,
    type RecurrenceRule,
    Segment,
    type Tick,
    TimeScale,
    TimeZone,
    Week,
    WEEK_DAYS,
} from "@destack/schema";
import type * as style from "@destack/style";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    type JSX,
    useContext,
    type useEnvironment,
    type useLocale,
    untrack,
    createMemo,
} from "@destack/view";
import { DragManager, type DragOperation, type DragTarget } from "../drag/index.ts";
import type { EventCalendarEventProperties } from "./event.tsx";
import { Focus, type KeyboardDelegate } from "../focus/index.ts";

/** The kind of item a calendar's draggable events are. */
export const EVENT_TYPE = "event";

/** The minutes a timed event snaps to by default, as Google's does. */
export const SNAP_MINUTES = 15;

/** The period each view moves by: a month, a week, or a day. */
const VIEW_PERIODS: Readonly<Record<EventCalendarView, (steps: number) => Period>> = {
    month: (steps) => ({ months: steps }),
    week: (steps) => ({ weeks: steps }),
    day: (steps) => ({ days: steps }),
    agenda: (steps) => ({ weeks: steps }),
};

/** The milliseconds of a day, from which a timed event shows among the all-day ones. */
const DAY = Duration.milliseconds({ days: 1 });

/** The events a day of a month shows at most by default, as FullCalendar's dayMaxEvents. */
const DAY_MAX_EVENTS = 3;

/** The milliseconds of an hour. */
const HOUR = Duration.milliseconds({ hours: 1 });

/** The group of the time grid's day columns, which a keyboard drag crosses between. */
export const COLUMNS = "event-calendar-columns";

/** The views a calendar offers, as Google Calendar and FullCalendar name them. */
export const EVENT_CALENDAR_VIEWS = ["month", "week", "day", "agenda"] as const;

/** A view of a calendar: a month of weeks, a week or a day of hours, or a list of days. */
export type EventCalendarView = (typeof EVENT_CALENDAR_VIEWS)[number];

/** How an event shows, after FullCalendar: as an event, or as shading behind the others. */
export type CalendarEventDisplay = "auto" | "background";

/** A change to one repeat of an event, after JSCalendar's recurrence overrides. */
export type RecurrenceOverride<Time> =
    | {
          /** The repeat does not happen. */
          readonly excluded: true;
      }
    | {
          /** The repeat's start. */
          readonly start: Time;
          /** The repeat's end. */
          readonly end: Time;
      };

/** The fields every event has, over whole days or over a time. */
interface CalendarEventFields<Time> {
    /** The event's key. */
    readonly key: string;
    /** The event's title. */
    readonly title: string;
    /** The start: the first day, or an instant in epoch milliseconds. */
    readonly start: Time;
    /** The end: the last day, or an instant at or after the start. */
    readonly end: Time;
    /** The event's color, blue by default. */
    readonly color?: AccentPreset;
    /** How the event shows, as an event by default. */
    readonly display?: CalendarEventDisplay;
    /** The rules it repeats by, from its first day, once when absent. */
    readonly recurrenceRules?: readonly RecurrenceRule[];
    /** The changes to single repeats, by the date each repeat would start on. */
    readonly recurrenceOverrides?: Readonly<Record<PlainDate, RecurrenceOverride<Time>>>;
}

/** An event after JSCalendar: over whole days from dates, or over a time from instants. */
export type CalendarEvent =
    | CalendarEventFields<PlainDate>
    | (CalendarEventFields<number> & {
          /** The IANA time zone its wall times repeat and move in, the calendar's by default. */
          readonly timeZone?: string;
      });

/** One time an event happens: the event itself, or one of its repeats. */
interface CalendarOccurrence {
    /** The event as it happens this time, keyed by its repeat. */
    readonly event: CalendarEvent;
    /** The repeating event, none for an event that happens once. */
    readonly of: CalendarEvent | undefined;
    /** The date the repeat would start on, which keys its override. */
    readonly recurrenceId: PlainDate;
}

/** An event over a time, its start and end instants. */
export type TimedCalendarEvent = Extract<CalendarEvent, { readonly start: number }>;

/** The events, view, date and handlers of a calendar. */
export interface EventCalendarOptions {
    /** The events. */
    readonly events: readonly CalendarEvent[];
    /** The view shown, which makes the view controlled. */
    readonly view?: EventCalendarView;
    /** The view shown at first while uncontrolled, the month by default. */
    readonly defaultView?: EventCalendarView;
    /** Handle the person choosing another view. */
    readonly onViewChange?: (view: EventCalendarView) => void;
    /** The date the view shows, which makes the date controlled. */
    readonly date?: PlainDate;
    /** The date shown at first while uncontrolled, today by default. */
    readonly defaultDate?: PlainDate;
    /** Handle the person moving to another date. */
    readonly onDateChange?: (date: PlainDate) => void;
    /** Handle the person moving or resizing an event, a moved repeat as that repeat's override. */
    readonly onEventChange?: (event: CalendarEvent) => void;
    /** Handle the person moving a repeat, which changes it, it and the later ones, or all. */
    readonly onRepeatChange?: (move: CalendarRepeatMove) => void;
    /** Handle the person opening an event, with the date of the repeat they opened. */
    readonly onAction?: (key: string, recurrenceId: PlainDate | undefined) => void;
    /** Handle the person sweeping out a new event over empty time. */
    readonly onCreate?: (span: TimeScale) => void;
    /** The current instant in epoch milliseconds, the environment's clock by default. */
    readonly now?: number;
    /** Whether the views show the locale's weekend days, true by default. */
    readonly weekends?: boolean;
    /** The IANA time zone the days and hours run in, the environment's by default. */
    readonly timeZone?: string;
    /** The events a day of the month view shows before a link to the rest, 3 by default. */
    readonly dayMaxEvents?: number;
    /** The minutes a timed event snaps to, 15 by default, fixed once the calendar mounts. */
    readonly slotDuration?: number;
    /** The hour the day and week views open at, 7 by default. */
    readonly openingHour?: number;
    /** The height of an hour in the day and week views, 3rem by default. */
    readonly hourHeight?: string;
}

/** The properties of a calendar, the native element's attributes included. */
export interface EventCalendarProperties
    extends EventCalendarOptions, Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> {
    /** The parts, a header and the view by default. */
    readonly children?: JSX.Element;
    /** The StyleX styles applied after the calendar's styles. */
    readonly xstyle?: style.Styles;
}

/** Render an event as a view lists it. */
export type EventCalendarEventRender = (properties: EventCalendarEventProperties) => JSX.Element;

/** The function the nearest view renders its events with, null for the calendar's default. */
export const EventCalendarEventContext = createContext<EventCalendarEventRender | null>(null);

/** The view, date, events and drags of a calendar, which its parts share. */
export class EventCalendarControl {
    /** The view shown. */
    readonly view: Accessor<EventCalendarView>;
    /** Show another view. */
    readonly setView: (view: EventCalendarView) => void;
    /** The date the view shows. */
    readonly date: Accessor<PlainDate>;
    /** Move to another date. */
    readonly setDate: (date: PlainDate) => void;
    /** The current instant, unknown until the view is mounted. */
    readonly now: Accessor<number | undefined>;
    /** Today in the calendar's time zone, unknown until the view is mounted. */
    readonly today: Accessor<PlainDate | undefined>;
    /** The time zone the days run in. */
    readonly timeZone: Accessor<string>;
    /** The weeks of the reader's locale. */
    readonly week: Week;
    /** The milliseconds a timed event snaps to. */
    readonly snap: number;
    /** The drags of the events. */
    readonly manager: DragManager;
    /** The focused day, which is the calendar's date. */
    readonly focus: Focus<string>;
    /** The calendar's options. */
    readonly options: EventCalendarOptions;
    /** The times the events happen in the shown range, by key. */
    readonly #occurrences: Accessor<ReadonlyMap<string, CalendarOccurrence>>;
    /** The clock today is read from when the person asks for it. */
    readonly #clock: ReturnType<typeof useEnvironment>["clock"];

    /** Follow a calendar's options in a locale's weeks, a time zone and a ticking clock. */
    constructor(
        options: EventCalendarOptions,
        locale: ReturnType<typeof useLocale>,
        environment: ReturnType<typeof useEnvironment>,
        ticking: Accessor<number | undefined>,
    ) {
        // read the zone and today in it
        this.options = options;
        this.week = locale.week;
        this.snap = untrack(() => options.slotDuration ?? SNAP_MINUTES) * 60_000;
        this.timeZone = () => options.timeZone ?? environment.timeZone;
        this.now = () => options.now ?? ticking();
        this.today = () => {
            const at = this.now();

            return at === undefined ? undefined : TimeZone.date(this.timeZone(), at);
        };
        this.#clock = environment.clock;
        const opening = untrack(() =>
            TimeZone.date(this.timeZone(), options.now ?? environment.clock.now()),
        );

        // follow the controlled view and date, or the calendar's
        [this.view, this.setView] = createControllableSignal<EventCalendarView>({
            isControlled: () => options.view !== undefined,
            value: () => options.view ?? "month",
            defaultValue: untrack(() => options.defaultView ?? "month"),
            onChange: (view) => options.onViewChange?.(view),
        });
        [this.date, this.setDate] = createControllableSignal<PlainDate>({
            isControlled: () => options.date !== undefined,
            value: () => options.date ?? opening,
            defaultValue: untrack(() => options.defaultDate ?? opening),
            onChange: (date) => options.onDateChange?.(date),
        });

        // move the focus through the days as the calendar's date
        this.focus = new Focus({
            delegate: new DayDelegate(this),
            mode: "roving",
            controlled: () => this.date(),
            onChange: (key) => this.setDate(key),
        });

        // expand the events into the times they happen in the shown range
        this.#occurrences = createMemo(() => {
            const range = this.range();

            return new Map(
                options.events
                    .flatMap((event) => this.#occurrencesOf(event, range))
                    .map((occurrence) => [occurrence.event.key, occurrence]),
            );
        });

        // move events by drags, reporting each drop as a change
        this.manager = new DragManager(
            { onDrop: (operation) => this.drop(operation) },
            locale,
            environment,
        );
    }

    /** Show today, read from the clock the moment the person asks. */
    showToday(): void {
        this.setDate(TimeZone.date(this.timeZone(), this.options.now ?? this.#clock.now()));
    }

    /** List the times the events happen in the shown range, each repeat its own event. */
    events(): CalendarEvent[] {
        return [...this.#occurrences().values()].map((occurrence) => occurrence.event);
    }

    /** Read the dates the view shows, its first and last, a month's whole weeks included. */
    range(): { readonly from: PlainDate; readonly to: PlainDate } {
        // read the month's grid, or the shown days of the other views
        if (this.view() === "month") {
            const grid = MonthGrid.of(PlainDate.with(this.date(), { day: 1 }), this.week, "fixed");
            const days = grid.weeks.flat();

            return { from: nth(days, 0), to: nth(days, -1) };
        }
        const days = this.days();

        return { from: nth(days, 0), to: nth(days, -1) };
    }

    /** Read a shown event by its key, a repeat's key included, refusing a key of no event. */
    eventOf(key: string): CalendarEvent {
        return this.#occurrenceOf(key).event;
    }

    /** Open an event, telling the handler which repeat. */
    act(key: string): void {
        const occurrence = this.#occurrenceOf(key);
        this.options.onAction?.(
            occurrence.of?.key ?? key,
            occurrence.of === undefined ? undefined : occurrence.recurrenceId,
        );
    }

    /** Hand on a moved event as a change: the event, or its repeating event with an override. */
    change(key: string, moved: CalendarEvent): void {
        // change an event that happens once as it is
        const { of, recurrenceId } = this.#occurrenceOf(key);
        if (of === undefined) {
            this.options.onEventChange?.(moved);
        }
        // hand a moved repeat to the handler that chooses what it changes
        else if (this.options.onRepeatChange !== undefined) {
            this.options.onRepeatChange(
                new CalendarRepeatMove(of, recurrenceId, this.eventOf(key), moved, this),
            );
        }
        // else override this repeat alone with its new times, after JSCalendar
        else {
            this.options.onEventChange?.(overridden(of, recurrenceId, moved));
        }
    }

    /** Read a shown occurrence by its key, refusing a key of none. */
    #occurrenceOf(key: string): CalendarOccurrence {
        const occurrence = this.#occurrences().get(key);
        if (occurrence === undefined) {
            throw new TypeError(`calendar event without an entry: '${key}'`);
        }

        return occurrence;
    }

    /** List the times an event happens in a range: once, or each repeat with its override. */
    #occurrencesOf(
        event: CalendarEvent,
        range: { readonly from: PlainDate; readonly to: PlainDate },
    ): CalendarOccurrence[] {
        // take an event that happens once as it is
        const span = this.spanOf(event);
        const rules = event.recurrenceRules;
        if (rules === undefined || rules.length === 0) {
            return [{ event, of: undefined, recurrenceId: span.start }];
        }

        // repeat it from its first day, reaching back by its length so a repeat into the range shows
        const length = PlainDate.daysUntil(span.start, span.end);
        const from = PlainDate.add(range.from, { days: -length });
        const zone = isTimed(event) ? (event.timeZone ?? this.timeZone()) : this.timeZone();
        const dates = Recurrence.dates(
            rules,
            this.#firstDayOf(event, zone),
            from,
            range.to,
            this.week,
        );

        // take each repeat but an excluded one, at its date and the event's wall times or its override's
        return dates.flatMap((date): CalendarOccurrence[] => {
            // leave out a repeat its override excludes
            const override = event.recurrenceOverrides?.[date];
            if (override !== undefined && "excluded" in override) {
                return [];
            }

            // move the event to the date, keyed by its repeat, then give it the override's times
            const days = PlainDate.daysUntil(this.#firstDayOf(event, zone), date);
            const repeat = { ...this.shift(event, days), key: `${event.key}@${date}` };
            const moved = override === undefined ? repeat : retimed(repeat, override);

            return [{ event: moved, of: event, recurrenceId: date }];
        });
    }

    /** Read the day an event starts on in the zone its wall times repeat in. */
    #firstDayOf(event: CalendarEvent, zone: string): PlainDate {
        return isTimed(event) ? TimeZone.date(zone, event.start) : event.start;
    }

    /** Read a timed event by its key, refusing a key of an event over whole days. */
    timedOf(key: string): TimedCalendarEvent {
        const event = this.eventOf(key);
        if (!isTimed(event)) {
            throw new TypeError(`calendar event over whole days read as timed: '${key}'`);
        }

        return event;
    }

    /** Read an event's days in the calendar's zone, its last day included. */
    spanOf(event: CalendarEvent): DateSpan {
        // read a timed event's days, its end belonging to the moment before it
        const zone = this.timeZone();
        if (isTimed(event)) {
            const last = Math.max(event.end - 1, event.start);

            return {
                key: event.key,
                start: TimeZone.date(zone, event.start),
                end: TimeZone.date(zone, last),
            };
        }

        return { key: event.key, start: event.start, end: event.end };
    }

    /** Move an event by whole days, at the same wall times in its zone across daylight saving. */
    shift(event: CalendarEvent, days: number): CalendarEvent {
        // move a timed event's ends to the same wall times days later
        const zone = isTimed(event) ? (event.timeZone ?? this.timeZone()) : this.timeZone();
        if (isTimed(event)) {
            const move = (at: number): number => {
                // read the wall time and the seconds past its minute
                const date = TimeZone.date(zone, at);
                const time = TimeZone.time(zone, at);
                const seconds = at - TimeZone.instant(zone, date, time);

                return TimeZone.instant(zone, PlainDate.add(date, { days }), time) + seconds;
            };

            return { ...event, start: move(event.start), end: move(event.end) };
        }

        return {
            ...event,
            start: PlainDate.add(event.start, { days }),
            end: PlainDate.add(event.end, { days }),
        };
    }

    /** List the days a week, day or agenda view shows: the date's shown week, or the date alone. */
    days(): PlainDate[] {
        // show the date alone in the day view, else its week from the locale's first day
        const date = this.date();
        if (this.view() === "day") {
            return [date];
        }
        const start = Week.startOf(this.week, date);
        const week = Array.from({ length: WEEK_DAYS }, (_, index) =>
            PlainDate.add(start, { days: index }),
        );

        return week.filter((day) => !this.isHidden(day));
    }

    /** Report whether the views hide a day: a weekend day while weekends are off. */
    isHidden(day: PlainDate): boolean {
        return this.options.weekends === false && Week.isWeekend(this.week, day);
    }

    /** Move a hidden day to the next shown one in a direction, a shown day staying put. */
    skipHidden(day: PlainDate, step: 1 | -1): PlainDate {
        // step at most a week, since every week shows a day
        let shown = day;
        for (let index = 0; index < WEEK_DAYS && this.isHidden(shown); index += 1) {
            shown = PlainDate.add(shown, { days: step });
        }

        return shown;
    }

    /** Cut the events to a row of shown days and stack them in levels, each on shown days. */
    stack(spans: readonly DateSpan[], days: readonly PlainDate[]): Segment<PlainDate>[] {
        // move each segment's ends off hidden days, leaving out one on hidden days alone
        return Segment.stack(spans, days)
            .map((segment) => ({
                ...segment,
                start: this.skipHidden(segment.start, 1),
                end: this.skipHidden(segment.end, -1),
            }))
            .filter((segment) => PlainDate.compare(segment.start, segment.end) <= 0);
    }

    /** Read the day an instant falls on in the calendar's zone. */
    dayOf(at: number): PlainDate {
        return TimeZone.date(this.timeZone(), at);
    }

    /** Read the events a day of the month view shows before a link to the rest. */
    dayMaxEvents(): number {
        return this.options.dayMaxEvents ?? DAY_MAX_EVENTS;
    }

    /** Read the id of a day's column in the time grid. */
    column(day: PlainDate): string {
        return `${COLUMNS}-${day}`;
    }

    /** Mark a day's local hours strictly inside it, leaving its two midnights to the borders. */
    hours(scale: TimeScale): Tick[] {
        // ask for one tick per hour, so a day of 23 or 25 hours has hourly marks
        const count = Math.round((scale.end - scale.start) / HOUR);

        return TimeScale.ticks(scale, count, this.timeZone(), this.week).filter(
            (tick) => tick.at > scale.start && tick.at < scale.end,
        );
    }

    /** Read a day's span of time in the calendar's zone, from its midnight to the next. */
    dayScale(day: PlainDate): TimeScale {
        // read both midnights, a daylight-saving day running 23 or 25 hours
        const zone = this.timeZone();

        return {
            start: TimeZone.instant(zone, day, "00:00"),
            end: TimeZone.instant(zone, PlainDate.add(day, { days: 1 }), "00:00"),
        };
    }

    /** Step the view's date by the view's length: a month, a week or a day. */
    step(steps: number): void {
        // move by the view's period, past the days a work week hides
        const date = PlainDate.add(this.date(), VIEW_PERIODS[this.view()](steps));
        this.setDate(this.skipHidden(date, steps < 0 ? -1 : 1));
    }

    /** Hand on a drop as a change: a move to another day or time, or a new end. */
    drop(operation: DragOperation & { readonly target: DragTarget }): void {
        // move the event by the days between the day it was taken by and the target's
        const { grab, target } = operation;
        if (grab.kind === "cell" && target.kind === "cell") {
            const days = PlainDate.daysUntil(grab.key, target.key);
            this.change(operation.item, this.shift(this.eventOf(operation.item), days));
        }
        // move a timed event's end, a snap after its start at least
        else if (target.kind === "time" && operation.kind === "resize-end") {
            const event = this.eventOf(operation.item);
            if (!isTimed(event)) {
                throw new TypeError(`calendar resize of an all-day event: '${event.key}'`);
            }
            this.change(operation.item, {
                ...event,
                end: Math.max(target.at, event.start + this.snap),
            });
        }
        // move a timed event by the time between where it was taken and the target
        else if (target.kind === "time" && grab.kind === "time") {
            const event = this.eventOf(operation.item);
            if (!isTimed(event)) {
                throw new TypeError(`calendar move of an all-day event into time: '${event.key}'`);
            }
            const shift = target.at - grab.at;
            this.change(operation.item, {
                ...event,
                start: event.start + shift,
                end: event.end + shift,
            });
        }
        // refuse drops between other places
        else {
            throw new TypeError(
                `calendar drop between other places: ${grab.kind} to ${target.kind}`,
            );
        }
    }
}

/** A moved repeat of a repeating event, and what it may change, after Google Calendar's choice. */
export class CalendarRepeatMove {
    /** The repeating event. */
    readonly event: CalendarEvent;
    /** The date the moved repeat would start on. */
    readonly recurrenceId: PlainDate;
    /** The repeat where it stood. */
    readonly repeat: CalendarEvent;
    /** The repeat where the person moved it. */
    readonly moved: CalendarEvent;
    /** The calendar, whose zone and weeks the changes follow. */
    readonly #control: EventCalendarControl;

    /** Describe a repeat's move from where it stood to where the person left it. */
    constructor(
        event: CalendarEvent,
        recurrenceId: PlainDate,
        repeat: CalendarEvent,
        moved: CalendarEvent,
        control: EventCalendarControl,
    ) {
        // keep the move and the calendar it happened in
        this.event = event;
        this.recurrenceId = recurrenceId;
        this.repeat = repeat;
        this.moved = moved;
        this.#control = control;
    }

    /** Change this repeat alone: the event with the repeat's override. */
    only(): CalendarEvent {
        return overridden(this.event, this.recurrenceId, this.moved);
    }

    /** Change every repeat: the event moved as this repeat moved, by days and by time of day. */
    all(): CalendarEvent {
        // move the event by the repeat's whole days at its wall times
        const control = this.#control;
        const days = PlainDate.daysUntil(
            control.spanOf(this.repeat).start,
            control.spanOf(this.moved).start,
        );
        const shifted = control.shift(this.event, days);

        // then by the repeat's change within its day, and its new length
        const { repeat, moved } = this;
        const lands = control.shift(repeat, days);
        if (isTimed(shifted) && isTimed(lands) && isTimed(moved)) {
            const start = shifted.start + moved.start - lands.start;

            return { ...shifted, start, end: start + moved.end - moved.start };
        }

        return shifted;
    }

    /** Change this repeat and the later ones: the event ended the day before, and a new one. */
    following(key: string): readonly [CalendarEvent, CalendarEvent] {
        // end the event's rules the day before this repeat, keeping the earlier overrides
        const control = this.#control;
        const rules = this.event.recurrenceRules ?? [];
        const first = control.spanOf(this.event).start;
        const { before, after } = Recurrence.split(rules, first, this.recurrenceId, control.week);
        const ended = withRepeats(this.event, before, (date) => date < this.recurrenceId);

        // go on from the moved repeat with the rules' repeats left and the later overrides
        const continued = withRepeats(
            { ...this.moved, key },
            after,
            (date) => date > this.recurrenceId,
            this.event,
        );

        return [ended, continued];
    }
}

/** The days a calendar's keys move through: a day along a week, a week across, a month by page. */
class DayDelegate implements KeyboardDelegate<string> {
    /** The calendar, whose weeks, date and hidden days the keys follow. */
    readonly #control: EventCalendarControl;

    /** Move through a calendar's shown days. */
    constructor(control: EventCalendarControl) {
        this.#control = control;
    }

    /** Read the day a key moves the focus to, left and right mirrored in right-to-left text. */
    target(event: KeyboardEvent, from: string, direction: "ltr" | "rtl"): string | undefined {
        // step along the reading direction, across weeks, or to the week's ends
        const control = this.#control;
        const start = Week.startOf(control.week, from);
        if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
            const isForward = (event.key === "ArrowRight") !== (direction === "rtl");
            const step = isForward ? 1 : -1;

            return control.skipHidden(PlainDate.add(from, { days: step }), step);
        } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
            return PlainDate.add(from, { weeks: event.key === "ArrowDown" ? 1 : -1 });
        } else if (event.key === "Home") {
            return control.skipHidden(start, 1);
        } else if (event.key === "End") {
            return control.skipHidden(PlainDate.add(start, { days: WEEK_DAYS - 1 }), -1);
        }
        // page by months, or by years with shift
        else if (event.key === "PageDown" || event.key === "PageUp") {
            const steps = event.key === "PageDown" ? 1 : -1;
            const paged = PlainDate.add(
                from,
                event.shiftKey ? { years: steps } : { months: steps },
            );

            return control.skipHidden(paged, steps);
        }

        return undefined;
    }

    /** Read the day the focus starts on: the calendar's date. */
    first(): string {
        return this.#control.date();
    }

    /** Report whether a key is a day the calendar shows. */
    has(key: string): boolean {
        const parsed = PlainDate.safeParse(key);

        return parsed.success && !this.#control.isHidden(parsed.data);
    }
}

/** The nearest calendar, null outside one. */
export const EventCalendarContext = createContext<EventCalendarControl | null>(null);

/** Report whether an event runs over a time rather than whole days. */
export function isTimed(event: CalendarEvent): event is TimedCalendarEvent {
    return typeof event.start === "number";
}

/** Report whether an event shows among the all-day ones: over whole days, or a day long or more. */
export function isAllDay(event: CalendarEvent): boolean {
    return !isTimed(event) || event.end - event.start >= DAY;
}

/** Read the columns a segment takes in a row of shown days: its first from zero, and how many. */
export function columnsOf(
    segment: Segment<PlainDate>,
    days: readonly PlainDate[],
): { readonly start: number; readonly span: number } {
    const start = days.indexOf(segment.start);

    return { start, span: days.indexOf(segment.end) - start + 1 };
}

/** Give an event a repeat's override of its times, the times matching the event's kind. */
function retimed(
    event: CalendarEvent,
    override: { readonly start: PlainDate | number; readonly end: PlainDate | number },
): CalendarEvent {
    // take instants for a timed event and dates for one over whole days
    const { start, end } = override;
    if (isTimed(event) && typeof start === "number" && typeof end === "number") {
        return { ...event, start, end };
    } else if (!isTimed(event) && typeof start === "string" && typeof end === "string") {
        return { ...event, start, end };
    }
    throw new TypeError(
        `calendar override of '${event.key}' is invalid: its times are another kind`,
    );
}

/** Give an event rules and another event's overrides of the dates a filter keeps. */
function withRepeats(
    event: CalendarEvent,
    rules: readonly RecurrenceRule[],
    keeps: (date: PlainDate) => boolean,
    from: CalendarEvent = event,
): CalendarEvent {
    // keep instants for a timed event and dates for one over whole days
    if (isTimed(event) && isTimed(from)) {
        return {
            ...event,
            recurrenceRules: rules,
            recurrenceOverrides: kept(from.recurrenceOverrides, keeps),
        };
    } else if (!isTimed(event) && !isTimed(from)) {
        return {
            ...event,
            recurrenceRules: rules,
            recurrenceOverrides: kept(from.recurrenceOverrides, keeps),
        };
    }
    throw new TypeError(`calendar repeats of '${from.key}' moved onto another kind of event`);
}

/** Keep the overrides of the dates a filter keeps. */
function kept<Time>(
    overrides: Readonly<Record<PlainDate, RecurrenceOverride<Time>>> | undefined,
    keeps: (date: PlainDate) => boolean,
): Record<PlainDate, RecurrenceOverride<Time>> {
    return Object.fromEntries(Object.entries(overrides ?? {}).filter(([date]) => keeps(date)));
}

/** Add a moved repeat's times to its repeating event as the override of that repeat. */
function overridden(
    event: CalendarEvent,
    recurrenceId: PlainDate,
    moved: CalendarEvent,
): CalendarEvent {
    // override a timed event with instants, and one over whole days with dates
    if (isTimed(event) && isTimed(moved)) {
        const override = { start: moved.start, end: moved.end };

        return {
            ...event,
            recurrenceOverrides: { ...event.recurrenceOverrides, [recurrenceId]: override },
        };
    } else if (!isTimed(event) && !isTimed(moved)) {
        const override = { start: moved.start, end: moved.end };

        return {
            ...event,
            recurrenceOverrides: { ...event.recurrenceOverrides, [recurrenceId]: override },
        };
    }
    throw new TypeError(`calendar repeat of '${event.key}' moved into another kind of event`);
}

/** Report whether an event shows as shading behind the others. */
export function isBackground(event: CalendarEvent): boolean {
    return event.display === "background";
}

/** Read the nearest calendar, refusing a part outside one. */
export function useEventCalendar(): EventCalendarControl {
    const control = useContext(EventCalendarContext);
    if (control === null) {
        throw new TypeError("an event calendar part needs an event calendar around it");
    }

    return control;
}
