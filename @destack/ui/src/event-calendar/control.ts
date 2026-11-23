import {
    nth,
    type DateSpan,
    Duration,
    MonthGrid,
    type Period,
    PlainDate,
    Segment,
    type Tick,
    TimeScale,
    TimeZone,
    Week,
    WEEK_DAYS,
    WorkCalendar,
    WorkWeek,
    PlainTime,
    present,
} from "@destack/schema";
import { type Message, t } from "@destack/locale";
import type * as style from "@destack/style";
import {
    flush,
    handle,
    type Accessor,
    createContext,
    createControllableSignal,
    type JSX,
    useContext,
    type useEnvironment,
    type useLocale,
    untrack,
    createMemo,
    createSignal,
    createUniqueId,
} from "@destack/view";
import type { Action } from "@destack/object";
import {
    clearSelection,
    createItem,
    duplicateItems,
    openItem,
    removeItems,
    selectAll,
} from "../action/index.ts";
import {
    clearAvailability,
    copyAvailability,
    showNext,
    showPrevious,
    showToday,
    VIEW_ACTIONS,
} from "./actions.ts";
import { writeClipboard } from "@destack/view/primitives/clipboard";
import type { Draggable, DragManager, DragOperation, DragTarget } from "../drag/index.ts";
import type { InteractionEvent } from "../interaction/index.ts";
import { type MultipleSelection, Selection } from "../selection/index.ts";
import type { EventCalendarEventProperties } from "./chip.tsx";
import {
    CalendarEvent,
    type CalendarEventDraft,
    type CalendarOccurrence,
    type TimedCalendarEvent,
} from "./event.ts";
import { CalendarRepeatChange, CalendarRepeatRemoval } from "./repeat.ts";
import { CalendarAvailability, type CalendarAvailabilityOptions } from "./availability.ts";
import type { CalendarResource, CalendarSource } from "./source.ts";
import type { Hue } from "@destack/color";
import { Focus, type KeyboardDelegate } from "../focus/index.ts";
import { CollectionBuilder } from "../collection/index.ts";

/** The kind of item a calendar's draggable events are. */
export const EVENT_TYPE = "event";

/** The minutes a timed event snaps to by default. */
const SNAP_MINUTES = 15;

/** The minutes between the time grid's lines by default: an hour. */
const SLOT_MINUTES = 60;

/** The days the days view shows by default. */
export const DAY_COUNT = 4;

/** The time the day and week views open scrolled to by default, the start of a working day. */
const SCROLL_TIME: PlainTime = "07:00";

/** The period each view but the days view moves by, which steps by its shown days. */
const VIEW_PERIODS: Readonly<
    Record<Exclude<EventCalendarView, "days">, (steps: number) => Period>
> = {
    day: (steps) => ({ days: steps }),
    week: (steps) => ({ weeks: steps }),
    month: (steps) => ({ months: steps }),
    year: (steps) => ({ years: steps }),
    agenda: (steps) => ({ weeks: steps }),
};

/** The title of each view but the days view, which its count of days titles. */
const VIEW_TITLES: Readonly<Record<Exclude<EventCalendarView, "days">, Message>> = {
    day: t`Day`,
    week: t`Week`,
    month: t`Month`,
    year: t`Year`,
    agenda: t`Agenda`,
};

/** The color of an event whose calendar sets none. */
export const DEFAULT_COLOR: Hue = "blue";

/** The events a day of a month shows at most by default, as FullCalendar's dayMaxEvents. */
const DAY_MAX_EVENTS = 3;

/** The milliseconds of a minute. */
const MINUTE = 60_000;

/** The minutes of a day on the wall clock. */
const DAY_MINUTES = 24 * 60;

/** The milliseconds of a day on the wall clock, which the day views place times along. */
export const DAY_MILLISECONDS = DAY_MINUTES * MINUTE;

/** The milliseconds of an hour. */
const HOUR = Duration.milliseconds({ hours: 1 });

/** The group of the time grid's day columns, which a keyboard drag crosses between. */
export const COLUMNS = "event-calendar-columns";

/** The views a calendar offers, as in FullCalendar, shortest first. */
export const EVENT_CALENDAR_VIEWS = ["day", "days", "week", "month", "year", "agenda"] as const;

/** A view of a calendar: days of hours, a month, a year, or a list of days. */
export type EventCalendarView = (typeof EVENT_CALENDAR_VIEWS)[number];

/** The events, view, date and handlers of a calendar. */
export interface EventCalendarOptions extends CalendarAvailabilityOptions {
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
    readonly onRepeatChange?: (move: CalendarRepeatChange) => void;
    /** Handle the person removing a repeat, which removes it, it and the later ones, or all. */
    readonly onRepeatRemove?: (removal: CalendarRepeatRemoval) => void;
    /** Handle the person opening an event, with the date of the repeat they opened. */
    readonly onAction?: (key: string, recurrenceId: PlainDate | undefined) => void;
    /** Handle the person sweeping out a new event over empty time. */
    readonly onCreate?: (span: TimeScale) => void;
    /** Handle the person adding an event, such as a copy dropped with Alt held or a duplicate. */
    readonly onEventAdd?: (event: CalendarEventDraft) => void;
    /** Handle the person removing an event that happens once. */
    readonly onEventRemove?: (event: CalendarEvent) => void;
    /** The current instant in epoch milliseconds, the environment's clock by default. */
    readonly now?: number;
    /** Whether the views show the days without working hours, true by default. */
    readonly weekends?: boolean;
    /** The working hours, which the day views shade around, the locale's weekdays by default. */
    readonly workWeek?: WorkWeek;
    /** The calendars the events belong to, which give them colors and may hide them. */
    readonly sources?: readonly CalendarSource[];
    /** The keys of the calendars hidden, which makes them controlled. */
    readonly hiddenSources?: readonly string[];
    /** The keys of the calendars hidden at first while uncontrolled, none by default. */
    readonly defaultHiddenSources?: readonly string[];
    /** Handle the person showing or hiding a calendar. */
    readonly onHiddenSourcesChange?: (keys: readonly string[]) => void;
    /** What a sweep over empty time does: create an event, or mark time free. */
    readonly mode?: EventCalendarMode;
    /** The kinds of items from other views the calendar takes, such as a board's cards. */
    readonly dropTypes?: readonly string[];
    /** Handle the person dropping another view's item on a time or a day, after FullCalendar. */
    readonly onReceive?: (drop: CalendarDrop) => void;
    /** The selection of events, several by modifier presses by default. */
    readonly selection?: MultipleSelection;
    /** The resources the day views give columns side by side, such as people, none by default. */
    readonly resources?: readonly CalendarResource[];
    /** The days the days view shows, 4 by default. */
    readonly dayCount?: number;
    /** Whether the month and time views show the week of the year, after FullCalendar. */
    readonly weekNumbers?: boolean;
    /** The zones the day views write hours in beside the calendar's zone. */
    readonly timeZones?: readonly CalendarTimeZone[];
    /** The IANA time zone the days and hours run in, the environment's by default. */
    readonly timeZone?: string;
    /** The events a day of the month view shows before a link to the rest, 3 by default. */
    readonly dayMaxEvents?: number;
    /** The minutes between the time grid's lines, 60 by default, after FullCalendar's slots. */
    readonly slotDuration?: number;
    /** The minutes a timed event snaps to, 15 by default, fixed once the calendar mounts. */
    readonly snapDuration?: number;
    /** The time the day views open scrolled to, 07:00 by default, after FullCalendar. */
    readonly scrollTime?: PlainTime;
    /** The height of an hour in the day and week views, 3rem by default. */
    readonly hourHeight?: string;
}

/** A zone a calendar writes its hours in beside its zone, such as a teammate's. */
export interface CalendarTimeZone {
    /** The IANA time zone. */
    readonly zone: string;
    /** The label above its hours, the zone's short name by default. */
    readonly label?: string;
}

/** What a sweep over a calendar's empty time does: create an event, or mark time free to share. */
export type EventCalendarMode = "events" | "availability";

/** An item of another view dropped on a calendar: a time in the day views, or a day. */
export interface CalendarDrop {
    /** The dropped item's key. */
    readonly item: string;
    /** The kind of item, one of the calendar's drop types. */
    readonly type: string;
    /** Where it landed: an instant in the time grid, or a day. */
    readonly start: number | PlainDate;
    /** The resource whose column it landed in, none without resources. */
    readonly resource?: string;
}

/** A shown day of a view, its place among its row's columns, and the resource it shows. */
export interface CalendarDay {
    /** The day. */
    readonly day: PlainDate;
    /** Its place among the row's columns, from 0. */
    readonly index: Accessor<number>;
    /** The resource whose column it is, none without resources. */
    readonly resource?: string;
}

/** The shown day a heading or column part renders. */
export const EventCalendarDayContext = createContext<CalendarDay | null>(null);

/** Read the shown day a heading or column part renders. */
export function useEventCalendarDay(): CalendarDay {
    const day = useContext(EventCalendarDayContext);
    if (day === null) {
        throw new TypeError("a calendar day part needs a row of days around it");
    }

    return day;
}

/** The properties of a part listing the shown days. */
export interface EventCalendarDaysProperties {
    /** Render a day's part, the list's default part when absent. */
    readonly children?: (day: CalendarDay) => JSX.Element;
}

/** The properties of a part of a calendar, the native element's attributes included. */
export interface EventCalendarPartProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class"
> {
    /** The StyleX styles applied after the part's. */
    readonly xstyle?: style.Styles;
}

/** The properties of a calendar, the native element's attributes included. */
export interface EventCalendarProperties
    extends EventCalendarOptions, Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> {
    /** The parts, such as a header and the views. */
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
    /** The working hours of the calendar's zone, which hidden days and shading follow. */
    readonly calendar: Accessor<WorkCalendar>;
    /** The selected events, which move, copy and remove together. */
    readonly selection: Selection;
    /** The time marked free to share. */
    readonly availability: CalendarAvailability;
    /** The key of the event the preview card shows, the last a pointer rested on or focused. */
    readonly previewed: Accessor<string | undefined>;
    /** Set the event the preview card shows. */
    readonly setPreviewed: (key: string | undefined) => void;
    /** The instant under the pointer in the day views, which every zone's hours mark. */
    readonly hovered: Accessor<number | undefined>;
    /** Set the instant under the pointer, none once it leaves. */
    readonly setHovered: (at: number | undefined) => void;
    /** The keys of the hidden calendars. */
    readonly hiddenSources: Accessor<readonly string[]>;
    /** Show or hide calendars by key. */
    readonly setHiddenSources: (keys: readonly string[]) => void;
    /** The views the calendar's parts offer, in the order of the calendar's views. */
    readonly views: Accessor<readonly EventCalendarView[]>;
    /** The views each mounted view part offers. */
    readonly #offered: CollectionBuilder<readonly EventCalendarView[]>;
    /** The drags of the events. */
    readonly manager: DragManager;
    /** The calendar's id, which tells its draggables and droppables from another calendar's. */
    readonly #id: string;
    /** The focused day, which is the calendar's date. */
    readonly focus: Focus<string>;
    /** The calendar's options. */
    readonly options: EventCalendarOptions;
    /** The times the events happen in the shown range, by key. */
    readonly #occurrences: Accessor<ReadonlyMap<string, CalendarOccurrence>>;
    /** The shown events, those of hidden calendars left out. */
    readonly #events: Accessor<readonly CalendarEvent[]>;
    /** The keys of the shown events. */
    readonly #shown: Accessor<ReadonlySet<string>>;
    /** The shown timed events within days, by day and resource, which each column reads. */
    readonly #timed: Accessor<ReadonlyMap<string, readonly TimedCalendarEvent[]>>;
    /** The working hours of each weekday, the owner's or the locale's weekdays whole. */
    readonly #workWeek: Accessor<WorkWeek>;
    /** Read the calendar's element. */
    #root: HTMLElement | undefined;
    /** The clock today is read from when the person asks for it. */
    readonly #clock: ReturnType<typeof useEnvironment>["clock"];

    /** Follow a calendar's options in a locale's weeks, a time zone and a ticking clock. */
    constructor(
        options: EventCalendarOptions,
        manager: DragManager,
        locale: ReturnType<typeof useLocale>,
        environment: ReturnType<typeof useEnvironment>,
        ticking: Accessor<number | undefined>,
    ) {
        // read the calendar's id, the zone and today in it
        this.#id = createUniqueId();
        this.options = options;
        this.week = locale.week;
        this.timeZone = () => options.timeZone ?? environment.timeZone;
        this.now = () => options.now ?? ticking();
        this.today = createMemo(() => {
            const at = this.now();

            return at === undefined ? undefined : TimeZone.date(this.timeZone(), at);
        });
        this.#clock = environment.clock;
        this.#workWeek = createMemo(() => options.workWeek ?? WorkWeek.of(locale.week.weekend));
        this.calendar = createMemo(() => new WorkCalendar(this.timeZone(), this.#workWeek()));
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
        const [previewed, setPreviewed] = createSignal<string | undefined>(undefined);
        this.previewed = previewed;
        this.setPreviewed = setPreviewed;
        const [hovered, setHovered] = createSignal<number | undefined>(undefined);
        this.hovered = hovered;
        this.setHovered = setHovered;
        this.availability = new CalendarAvailability(options, locale, this.timeZone);
        [this.hiddenSources, this.setHiddenSources] = createControllableSignal<readonly string[]>({
            isControlled: () => options.hiddenSources !== undefined,
            value: () => options.hiddenSources ?? [],
            defaultValue: untrack(() => options.defaultHiddenSources ?? []),
            onChange: (keys) => options.onHiddenSourcesChange?.(keys),
        });
        [this.date, this.setDate] = createControllableSignal<PlainDate>({
            isControlled: () => options.date !== undefined,
            value: () => options.date ?? opening,
            defaultValue: untrack(() => options.defaultDate ?? opening),
            onChange: (date) => options.onDateChange?.(date),
        });

        // list the views the mounted view parts offer
        this.#offered = new CollectionBuilder();
        this.views = () =>
            EVENT_CALENDAR_VIEWS.filter((view) =>
                this.#offered.items().some((views) => views.includes(view)),
            );

        // select several events at once
        this.selection = new Selection(
            untrack(() => options.selection) ?? { multiple: true, behavior: "replace" },
        );

        // move the focus through the days as the calendar's date
        this.focus = new Focus({
            delegate: new DayDelegate(this),
            mode: "roving",
            controlled: () => this.date(),
            onChange: (key) => this.setDate(key),
        });

        // expand the events into the times they happen in the range, again only for changed ones
        const expansion = createMemo((previous: Expansion | undefined): Expansion => {
            // reuse each unchanged event's occurrences while the range and zone stay
            const range = this.range();
            const zone = this.timeZone();
            const key = `${range.from} ${range.to} ${zone}`;
            const reused =
                previous?.key === key
                    ? previous.byEvent
                    : new Map<CalendarEvent, readonly CalendarOccurrence[]>();
            const byEvent = new Map(
                options.events.map((event) => [
                    event,
                    reused.get(event) ?? CalendarEvent.occurrences(event, range, zone, this.week),
                ]),
            );
            const byKey = new Map(
                [...byEvent.values()]
                    .flat()
                    .map((occurrence) => [occurrence.event.key, occurrence]),
            );

            return { key, byEvent, byKey };
        }, undefined);
        this.#occurrences = () => expansion().byKey;

        // leave out the events of hidden calendars, and bucket the timed ones by day and resource
        this.#events = createMemo(() => {
            const hidden = new Set(this.hiddenSources());

            return [...this.#occurrences().values()]
                .map((occurrence) => occurrence.event)
                .filter((event) => event.source === undefined || !hidden.has(event.source));
        });
        this.#shown = createMemo(() => new Set(this.#events().map((event) => event.key)));
        this.#timed = createMemo(() => {
            const buckets = new Map<string, TimedCalendarEvent[]>();
            for (const event of this.#events().filter(
                (each): each is TimedCalendarEvent =>
                    CalendarEvent.isTimed(each) && !CalendarEvent.isAllDay(each),
            )) {
                // add the event to each day it touches in its resource's columns
                const span = CalendarEvent.span(event, this.timeZone());
                const resource = this.resourceOf(event)?.key;
                for (
                    let day = span.start;
                    PlainDate.compare(day, span.end) <= 0;
                    day = PlainDate.add(day, { days: 1 })
                ) {
                    const bucket = this.column(day, resource);
                    const events = buckets.get(bucket) ?? [];
                    events.push(event);
                    buckets.set(bucket, events);
                }
            }

            return buckets;
        });

        // move events by drags, reporting each drop as a change
        this.manager = manager;
    }

    /** Read the milliseconds a timed event snaps to. */
    get snap(): number {
        return (this.options.snapDuration ?? SNAP_MINUTES) * MINUTE;
    }

    /** Offer a view part's views until it unmounts. */
    addViews(views: readonly EventCalendarView[]): void {
        this.#offered.add(views);
    }

    /** Read the milliseconds between the time grid's lines. */
    slot(): number {
        return (this.options.slotDuration ?? SLOT_MINUTES) * MINUTE;
    }

    /** Read the days the days view shows. */
    dayCount(): number {
        return this.options.dayCount ?? DAY_COUNT;
    }

    /** Read the time the day views open scrolled to. */
    scrollTime(): PlainTime {
        return this.options.scrollTime ?? SCROLL_TIME;
    }

    /** Open a day in the day view, as a press on its date does. */
    open(day: PlainDate): void {
        this.setDate(day);
        this.setView("day");
    }

    /** Show today, read from the clock the moment the person asks. */
    showToday(): void {
        this.setDate(TimeZone.date(this.timeZone(), this.options.now ?? this.#clock.now()));
    }

    /** List the times the events happen in the shown range, each repeat a separate event. */
    events(): readonly CalendarEvent[] {
        return this.#events();
    }

    /** List the shown timed events within days that touch a day and a resource's column. */
    timedOn(day: PlainDate, resource: string | undefined): readonly TimedCalendarEvent[] {
        return this.#timed().get(this.column(day, resource)) ?? [];
    }

    /** Read the calendar an event belongs to, none without one, refusing a missing one. */
    sourceOf(event: CalendarEvent): CalendarSource | undefined {
        // leave an event without a calendar alone, and refuse one the calendars lack
        if (event.source === undefined) {
            return undefined;
        }
        const source = this.options.sources?.find((each) => each.key === event.source);
        if (source === undefined) {
            throw new RangeError(
                `calendar event '${event.key}' is invalid: its source '${event.source}' is missing`,
            );
        }

        return source;
    }

    /** Read the resource an event belongs to, none without resources, refusing a missing one. */
    resourceOf(event: CalendarEvent): CalendarResource | undefined {
        // leave events alone in a calendar without resources
        const resources = this.options.resources ?? [];
        const resource = resources.find((each) => each.key === event.resource);
        if (resources.length === 0) {
            return undefined;
        }
        // refuse an event without one of the calendar's resources
        else if (resource === undefined) {
            throw new RangeError(
                `calendar event '${event.key}' is invalid: its resource '${String(event.resource)}' is missing`,
            );
        }
        // else read the resource
        else {
            return resource;
        }
    }

    /** Read an event's color: the event's, else its calendar's, else the default. */
    colorOf(event: CalendarEvent): Hue {
        return event.color ?? this.sourceOf(event)?.color ?? DEFAULT_COLOR;
    }

    /** Report whether an event shows as busy time, its calendar showing no details. */
    isBusy(event: CalendarEvent): boolean {
        return this.sourceOf(event)?.display === "busy";
    }

    /** Show or hide a calendar. */
    toggleSource(key: string): void {
        const hidden = this.hiddenSources();
        this.setHiddenSources(
            hidden.includes(key) ? hidden.filter((each) => each !== key) : [...hidden, key],
        );
    }

    /** List the time grid's columns: each shown day, or each resource's shown days in turn. */
    columns(): Omit<CalendarDay, "index">[] {
        const days = this.days();
        const resources = this.options.resources ?? [];

        return resources.length === 0
            ? days.map((day) => ({ day }))
            : resources.flatMap((resource) => days.map((day) => ({ day, resource: resource.key })));
    }

    /** Read the dates the view shows, its first and last, a month's whole weeks included. */
    range(): { readonly from: PlainDate; readonly to: PlainDate } {
        // read the month's grid, the year's days, or the shown days of the other views
        const view = this.view();
        if (view === "month") {
            const grid = MonthGrid.of(PlainDate.with(this.date(), { day: 1 }), this.week, "fixed");
            const days = grid.weeks.flat();

            return { from: nth(days, 0), to: nth(days, -1) };
        } else if (view === "year") {
            const first = PlainDate.with(this.date(), { month: 1, day: 1 });

            return { from: first, to: PlainDate.add(first, { years: 1, days: -1 }) };
        } else {
            const days = this.days();

            return { from: nth(days, 0), to: nth(days, -1) };
        }
    }

    /** Report whether an event shows in the range, a repeat's key included. */
    hasEvent(key: string): boolean {
        return this.#shown().has(key);
    }

    /** Read a shown event by its key, a repeat's key included, refusing a key of no event. */
    eventOf(key: string): CalendarEvent {
        return this.#occurrenceOf(key).event;
    }

    /** Open a pressed event, or add it to the selection or take it out with a modifier. */
    press(key: string, gesture: InteractionEvent): void {
        // add or take out an event by a press with shift, command or control
        const isModified = gesture.shiftKey || gesture.metaKey || gesture.ctrlKey;
        if (isModified && gesture.pointerType !== "keyboard") {
            this.selection.toggle(key);
        }
        // select alone by a plain press on a calendar that opens nothing
        else if (this.options.onAction === undefined) {
            this.selection.replace([key]);
        }
        // open the event
        else {
            this.act(key);
        }
    }

    /** Open an event, telling the handler which repeat. */
    act(key: string): void {
        const occurrence = this.#occurrenceOf(key);
        this.options.onAction?.(
            occurrence.of?.key ?? key,
            occurrence.of === undefined ? undefined : occurrence.recurrenceId,
        );
    }

    /** Hand on a changed event: one happening once, or a repeat to its choice or as an override. */
    change(key: string, moved: CalendarEvent): void {
        // change an event that happens once as it is
        const { of, recurrenceId } = this.#occurrenceOf(key);
        if (of === undefined) {
            this.#handler("onEventChange")(moved);
        }
        // hand a moved repeat to the handler that chooses what it changes
        else if (this.options.onRepeatChange !== undefined) {
            this.options.onRepeatChange(
                new CalendarRepeatChange(
                    of,
                    recurrenceId,
                    this.eventOf(key),
                    moved,
                    this.timeZone(),
                    this.week,
                ),
            );
        }
        // else override this repeat alone with its new times, after JSCalendar
        else {
            this.#handler("onEventChange")(CalendarEvent.override(of, recurrenceId, moved));
        }
    }

    /** Report whether the calendar can hand on changes of events, a lone one or several. */
    canChange(keys: readonly string[]): boolean {
        // change events that happen once, and repeats by their choice alone or as overrides
        const options = this.options;
        const hasRepeat = keys.some((key) => this.#occurrenceOf(key).of !== undefined);
        const hasOnce = keys.some((key) => this.#occurrenceOf(key).of === undefined);
        const canRepeat =
            keys.length === 1
                ? options.onRepeatChange !== undefined || options.onEventChange !== undefined
                : options.onEventChange !== undefined;

        return (
            keys.length > 0 &&
            (!hasOnce || options.onEventChange !== undefined) &&
            (!hasRepeat || canRepeat)
        );
    }

    /** Report whether the calendar can hand on removals of events, a lone one or several. */
    canRemove(keys: readonly string[]): boolean {
        // remove events that happen once, and repeats by their choice alone or as exclusions
        const options = this.options;
        const hasRepeat = keys.some((key) => this.#occurrenceOf(key).of !== undefined);
        const hasOnce = keys.some((key) => this.#occurrenceOf(key).of === undefined);
        const canRepeat =
            keys.length === 1
                ? options.onRepeatRemove !== undefined || options.onEventChange !== undefined
                : options.onEventChange !== undefined;

        return (
            keys.length > 0 &&
            (!hasOnce || options.onEventRemove !== undefined) &&
            (!hasRepeat || canRepeat)
        );
    }

    /** Read one of the calendar's handlers, refusing one it lacks, which its offers rule out. */
    #handler<Name extends "onEventChange" | "onEventRemove">(
        name: Name,
    ): NonNullable<EventCalendarOptions[Name]> {
        const handler = this.options[name];
        if (handler === undefined) {
            throw new TypeError(`calendar has no '${name}' handler`);
        }

        return handler;
    }

    /** Read a shown occurrence by its key, refusing a key of none. */
    #occurrenceOf(key: string): CalendarOccurrence {
        const occurrence = this.#occurrences().get(key);
        if (occurrence === undefined) {
            throw new TypeError(`calendar event '${key}' is not shown`);
        }

        return occurrence;
    }

    /** Read a timed event by its key, refusing a key of an event over whole days. */
    timedOf(key: string): TimedCalendarEvent {
        const event = this.eventOf(key);
        if (!CalendarEvent.isTimed(event)) {
            throw new TypeError(`calendar event over whole days read as timed: '${key}'`);
        }

        return event;
    }

    /** List the days a day, days, week or agenda view shows from the date. */
    days(): PlainDate[] {
        // show the date alone in the day view
        const date = this.date();
        const view = this.view();
        if (view === "day") {
            return [date];
        }
        // show the count of shown days from the date in the days view
        else if (view === "days") {
            const count = this.dayCount();
            const days: PlainDate[] = [];
            for (
                let day = this.skipHidden(date, 1);
                days.length < count;
                day = this.skipHidden(PlainDate.add(day, { days: 1 }), 1)
            ) {
                days.push(day);
            }

            return days;
        }
        // show the date's week from the locale's first day, past its hidden days
        else {
            const start = Week.startOf(this.week, date);
            const week = Array.from({ length: WEEK_DAYS }, (_, index) =>
                PlainDate.add(start, { days: index }),
            );

            return week.filter((day) => !this.isHidden(day));
        }
    }

    /** Report whether the views hide a day: a day without working hours while weekends are off. */
    isHidden(day: PlainDate): boolean {
        return this.options.weekends === false && this.isRestDay(day);
    }

    /** Report whether a day has no working hours, which the views tint. */
    isRestDay(day: PlainDate): boolean {
        return !WorkWeek.isWorkingDay(this.#workWeek(), PlainDate.dayOfWeek(day));
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

    /** Read the key a list renders a segment under, which a move, a cut or busy display changes. */
    segmentKey(segment: Segment<unknown>): string {
        const isBusy = this.isBusy(this.eventOf(segment.key));

        return JSON.stringify([segment.key, segment.start, segment.isStart, segment.isEnd, isBusy]);
    }

    /** Read the id of one of the calendar's draggables or droppables, unique among calendars. */
    dragId(name: string): string {
        return `${this.#id} ${name}`;
    }

    /** Read the id of a day's column in the time grid, a resource's when the grid has them. */
    column(day: PlainDate, resource: string | undefined): string {
        return this.dragId(JSON.stringify([COLUMNS, day, resource ?? null]));
    }

    /** Mark a day's local hours strictly inside it, leaving its two midnights to the borders. */
    hours(scale: TimeScale): Tick[] {
        // ask for one tick per hour, so a day of 23 or 25 hours has hourly marks
        const count = Math.round((scale.end - scale.start) / HOUR);

        return TimeScale.ticks(scale, count, this.timeZone(), this.week).filter(
            (tick) => tick.at > scale.start && tick.at < scale.end,
        );
    }

    /** List a day's grid lines strictly inside it: a line per slot, the hours' lines marked. */
    gridLines(scale: TimeScale): { readonly at: number; readonly isHour: boolean }[] {
        // mark every hour a slot of whole hours lands on
        const hours = this.hours(scale).map((tick) => tick.at);
        const slot = this.slot();
        if (slot % HOUR === 0) {
            const every = slot / HOUR;

            return hours
                .filter((_, index) => (index + 1) % every === 0)
                .map((at) => ({ at, isHour: true }));
        }

        // else add the slots between each hour and the next, from the day's first midnight
        const edges = [scale.start, ...hours, scale.end];

        return edges.slice(1).flatMap((end, index) => {
            const start = nth(edges, index);
            const slots = Array.from(
                { length: Math.ceil((end - start) / slot) - 1 },
                (_, each) => ({ at: start + (each + 1) * slot, isHour: false }),
            );

            return end < scale.end ? [...slots, { at: end, isHour: true }] : slots;
        });
    }

    /** Write the week of the year a day falls in, such as W47, in the locale's weeks. */
    weekNumber(day: PlainDate): Message {
        return t`W${String(Week.weekOfYear(this.week, day))}`;
    }

    /** Read an instant's place from a day's start on the wall clock, in days: 0.5 its noon. */
    wallShare(day: PlainDate, at: number): number {
        // count the days from the day, then the local minutes and the milliseconds past the minute
        const zone = this.timeZone();
        const days = PlainDate.daysUntil(day, TimeZone.date(zone, at));
        const minutes = PlainTime.minutes(TimeZone.time(zone, at));
        const past = ((at % MINUTE) + MINUTE) % MINUTE;

        return days + (minutes * MINUTE + past) / DAY_MILLISECONDS;
    }

    /** Read a span's place on a day's wall clock, in milliseconds into the day. */
    wallSpan(day: PlainDate, span: TimeScale): TimeScale {
        return {
            start: this.wallShare(day, span.start) * DAY_MILLISECONDS,
            end: this.wallShare(day, span.end) * DAY_MILLISECONDS,
        };
    }

    /** Read a wall span's top and height as shares of the day, a snap high at least. */
    wallBox(wall: TimeScale): { readonly top: number; readonly height: number } {
        const height = (wall.end - wall.start) / DAY_MILLISECONDS;

        return {
            top: wall.start / DAY_MILLISECONDS,
            height: Math.max(height, this.snap / DAY_MILLISECONDS),
        };
    }

    /** Read the instant at a share of a day on the wall clock, the reverse of its wall share. */
    wallInstant(day: PlainDate, share: number): number {
        // read the whole minutes as a local time, and add the rest
        const milliseconds = Math.min(Math.max(share, 0), 1) * DAY_MILLISECONDS;
        const minutes = Math.min(Math.floor(milliseconds / MINUTE), DAY_MINUTES - 1);
        const time = PlainTime.of(minutes);

        return TimeZone.instant(this.timeZone(), day, time) + milliseconds - minutes * MINUTE;
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

    /** Step the view's date by the view's length, the days view by its count of shown days. */
    step(steps: number): void {
        // move the days view by its count of shown days from its first shown day, past hidden ones
        const direction = steps < 0 ? -1 : 1;
        const view = this.view();
        if (view === "days") {
            let date = this.skipHidden(this.date(), 1);
            for (let index = 0; index < Math.abs(steps) * this.dayCount(); index += 1) {
                date = this.skipHidden(PlainDate.add(date, { days: direction }), direction);
            }
            this.setDate(date);
        }
        // move a month or a year, a hidden date moved to a shown one within the same period
        else if (view === "month" || view === "year") {
            const date = PlainDate.add(this.date(), VIEW_PERIODS[view](steps));
            const forward = this.skipHidden(date, 1);
            const isSameYear = PlainDate.year(forward) === PlainDate.year(date);
            const isSamePeriod =
                isSameYear &&
                (view === "year" || PlainDate.month(forward) === PlainDate.month(date));
            this.setDate(isSamePeriod ? forward : this.skipHidden(date, -1));
        }
        // move the other views by their period, past the days a work week hides
        else {
            const date = PlainDate.add(this.date(), VIEW_PERIODS[view](steps));
            this.setDate(this.skipHidden(date, direction));
        }
    }

    /** Hand on a drop: another view's item, or events moved or copied with the selection. */
    drop(operation: DragOperation & { readonly target: DragTarget }): void {
        // hand an item of another view on with the time or day it landed on
        const { target } = operation;
        if (operation.type !== EVENT_TYPE) {
            this.#receive(operation.item, operation.type, target);
        }
        // add copies where the person dropped them with Alt held, with the selected events
        else if (
            operation.effect === "copy" &&
            operation.kind === "move" &&
            this.options.onEventAdd !== undefined
        ) {
            for (const [, event] of this.#moved(operation)) {
                this.options.onEventAdd(CalendarEvent.draft(event));
            }
        }
        // move the event there, with the selected events, a series' repeats folded into one change
        else {
            this.#changeAll(this.#moved(operation));
        }
    }

    /** Hand an item of another view on with the time or day it landed on and its resource. */
    #receive(item: string, type: string, target: DragTarget): void {
        // refuse a target that is neither a time nor a day
        const start =
            target.kind === "time" ? target.at : target.kind === "cell" ? target.key : undefined;
        if (start === undefined) {
            throw new TypeError(`calendar drop of '${item}' onto a ${target.kind} target`);
        }
        const resource =
            target.kind === "time" || target.kind === "cell" ? target.resource : undefined;
        this.options.onReceive?.({
            item,
            type,
            start,
            ...(resource === undefined ? {} : { resource }),
        });
    }

    /** List the events a move of one takes: it alone, or the shown selection it belongs to. */
    #movingWith(key: string): string[] {
        return this.selection.isSelected(key)
            ? this.selection.values().filter((each) => this.hasEvent(each))
            : [key];
    }

    /** List the events a drop leaves: the dragged one, and the selected ones a move takes along. */
    #moved(
        operation: DragOperation & { readonly target: DragTarget },
    ): (readonly [string, CalendarEvent])[] {
        // move the dragged event, and the selected ones as far, at their wall times
        const outcome = this.outcome(operation);
        const original = this.eventOf(operation.item);
        const others =
            operation.kind === "move"
                ? this.#movingWith(operation.item)
                      .filter((key) => key !== operation.item)
                      .map(
                          (key) =>
                              [
                                  key,
                                  CalendarEvent.along(
                                      this.eventOf(key),
                                      original,
                                      outcome,
                                      this.timeZone(),
                                  ),
                              ] as const,
                      )
                : [];

        return [[operation.item, outcome], ...others];
    }

    /** Hand on changed events: a lone one as it is, several folded into a change per series. */
    #changeAll(changes: readonly (readonly [string, CalendarEvent])[]): void {
        // hand a lone change on as it is
        const [only] = changes;
        if (changes.length === 1 && only !== undefined) {
            this.change(only[0], only[1]);
            return;
        }

        // group the changes by the series they repeat, overriding each series' repeats in one change
        const groups = Map.groupBy(changes, ([key]) => this.#occurrenceOf(key).of ?? key);
        for (const [series, group] of groups) {
            const folded =
                typeof series === "string"
                    ? group.map(([, moved]) => moved)
                    : [
                          group.reduce(
                              (event, [key, moved]) =>
                                  CalendarEvent.override(
                                      event,
                                      this.#occurrenceOf(key).recurrenceId,
                                      moved,
                                  ),
                              series,
                          ),
                      ];
            folded.forEach((event) => this.#handler("onEventChange")(event));
        }
    }

    /** List the ids of the actions an event allows: those whose handler the calendar has. */
    #offers(key: string): ReadonlySet<string> {
        // offer nothing for a busy event or during a drag
        const options = this.options;
        if (this.isBusy(this.eventOf(key)) || this.manager.operation() !== undefined) {
            return new Set();
        }

        // offer opening, duplicating and removing by their handlers
        return new Set([
            ...(options.onAction === undefined ? [] : ["open"]),
            ...(options.onEventAdd === undefined ? [] : ["duplicate"]),
            ...(this.canRemove([key]) ? ["delete"] : []),
        ]);
    }

    /** Hand on a span swept over empty time in a column: a new event, or time marked free. */
    sweep(span: TimeScale, resource: string | undefined): void {
        if (this.options.mode === "availability") {
            this.availability.mark({ ...span, ...(resource === undefined ? {} : { resource }) });
        } else {
            this.options.onCreate?.(span);
        }
    }

    /** Find the shown element of an event, null while none shows. */
    elementOf(key: string): HTMLElement | null {
        return this.#root?.querySelector<HTMLElement>(`[data-event="${CSS.escape(key)}"]`) ?? null;
    }

    /** Remove events, the focus moving from a removed one to the shown event after or before it. */
    #removeFocused(keys: readonly string[]): void {
        // find the nearest event left, forward before back
        const shown = [
            ...(this.#root?.querySelectorAll<HTMLElement>("[data-event]") ?? []),
        ].flatMap(({ dataset }) => dataset["event"] ?? []);
        const at = shown.findIndex((key) => keys.includes(key));
        const isLeft = (key: string) => !keys.includes(key);
        const next = shown.slice(at).find(isLeft) ?? shown.slice(0, at).findLast(isLeft);

        // remove them, then focus it once the focused one is gone
        this.remove(keys);
        queueMicrotask(() => {
            flush();
            const { activeElement, body } = document;
            if (next !== undefined && (activeElement === null || activeElement === body)) {
                this.elementOf(next)?.focus();
            }
        });
    }

    /** Read the key of the event around an element, absent outside one. */
    eventAt(element: Element | null): string | undefined {
        return element?.closest<HTMLElement>("[data-event]")?.dataset["event"];
    }

    /** List the events an action at an element takes: its event, else the selection. */
    targetsAt(element: Element | null): string[] {
        // take the selection an event is in with others, else the event alone
        const key = this.eventAt(element);
        const selected = this.selection.values().filter((each) => this.hasEvent(each));
        const isGrouped = key !== undefined && selected.includes(key) && selected.length > 1;

        return key === undefined || isGrouped ? selected : [key];
    }

    /** List what handles the actions on the calendar and its events, for its element's `ref`. */
    handleActions(): ((element: HTMLElement) => void)[] {
        // handle an action outside drags, whose keys move the event
        const on = (
            action: Action,
            isEnabled: (element: Element | null) => boolean,
            run: (element: Element | null) => void,
        ) =>
            handle(action, {
                isEnabled: (element) =>
                    this.manager.operation() === undefined && isEnabled(element),
                run: ({ element }) => run(element),
            });
        // offer an action every event it takes offers
        const offers = (id: string) => (element: Element | null) => {
            const targets = this.targetsAt(element);

            return targets.length > 0 && targets.every((key) => this.#offers(key).has(id));
        };
        const open = offers("open");
        const isRemovable = offers("delete");
        const isMarked = () =>
            this.options.mode === "availability" && this.availability.spans().length > 0;

        return [
            // find events in the calendar's element
            (element) => {
                this.#root = element;
            },

            // move through dates, and switch among the offered views
            on(
                showNext,
                () => true,
                () => this.step(1),
            ),
            on(
                showPrevious,
                () => true,
                () => this.step(-1),
            ),
            on(
                showToday,
                () => true,
                () => this.showToday(),
            ),
            ...EVENT_CALENDAR_VIEWS.map((view) =>
                on(
                    VIEW_ACTIONS[view],
                    () => this.views().includes(view),
                    () => this.setView(view),
                ),
            ),

            // create events, and select them
            on(
                createItem,
                () => this.options.onCreate !== undefined,
                () => this.createNow(),
            ),
            on(
                selectAll,
                () => true,
                () => {
                    const events = this.events().filter(
                        (event) => !this.isBusy(event) && !CalendarEvent.isBackground(event),
                    );
                    this.selection.replace(events.map((event) => event.key));
                },
            ),
            on(
                clearSelection,
                () => this.selection.values().length > 0,
                () => this.selection.clear(),
            ),

            // open, duplicate and remove the events, by the actions each offers
            on(
                openItem,
                (element) => this.eventAt(element) !== undefined && open(element),
                (element) => this.act(present(this.eventAt(element), "the event to open")),
            ),
            on(duplicateItems, offers("duplicate"), (element) =>
                this.targetsAt(element).forEach((key) => this.duplicate(key)),
            ),
            on(
                removeItems,
                (element) => isRemovable(element) && this.canRemove(this.targetsAt(element)),
                (element) => this.#removeFocused(this.targetsAt(element)),
            ),

            // copy the free time while marking it
            on(copyAvailability, isMarked, () => {
                void writeClipboard(this.availability.text());
            }),
            on(clearAvailability, isMarked, () => this.availability.clear()),
        ];
    }

    /** List the zones the day views write hours in, the calendar's zone last. */
    zones(): { readonly zone: string; readonly label: string | undefined }[] {
        return [
            ...(this.options.timeZones ?? []).map((each) => ({
                zone: each.zone,
                label: each.label,
            })),
            { zone: this.timeZone(), label: undefined },
        ];
    }

    /** Read the title of a view, the days view by its count of days. */
    viewTitle(view: EventCalendarView): Message {
        const count = this.dayCount();

        return view === "days" ? t`${String(count)} days` : VIEW_TITLES[view];
    }

    /** Create an hour-long event from the next snap today, else from the date's scroll time. */
    createNow(): void {
        // start at today's next snap while the date is today, else at the scroll time
        const zone = this.timeZone();
        const now = this.now();
        const midnight = this.dayScale(this.date()).start;
        const start =
            now !== undefined && PlainDate.equals(this.date(), TimeZone.date(zone, now))
                ? midnight + Math.ceil((now - midnight) / this.snap) * this.snap
                : TimeZone.instant(zone, this.date(), this.scrollTime());
        this.options.onCreate?.({ start, end: start + HOUR });
    }

    /** Report whether a place of a kind takes a dragged item: a shown event, or a received type. */
    accepts(draggable: Draggable, place: "time" | "cell"): boolean {
        // take another view's items of the types the calendar receives
        if (draggable.type !== EVENT_TYPE) {
            return (
                this.options.onReceive !== undefined &&
                (this.options.dropTypes ?? []).includes(draggable.type)
            );
        }
        // refuse an event the calendar does not show
        else if (!this.hasEvent(draggable.item)) {
            return false;
        }
        // take events the calendar can hand on with the selection a move takes along,
        //  moved between the hours and the days only whole and once
        else {
            const isCrossing = draggable.home().kind !== place;
            const isRepeat = this.#occurrenceOf(draggable.item).of !== undefined;
            const moving =
                draggable.kind === "move" ? this.#movingWith(draggable.item) : [draggable.item];

            return (
                this.canChange(moving) && (!isCrossing || (draggable.kind === "move" && !isRepeat))
            );
        }
    }

    /** Add a copy of an event at its times. */
    duplicate(key: string): void {
        this.options.onEventAdd?.(CalendarEvent.draft(this.eventOf(key)));
    }

    /** Remove events: a lone repeat by its choice, several folded into a change per series. */
    remove(keys: readonly string[]): void {
        // hand a lone removed repeat to the handler that chooses what it removes
        const [only] = keys;
        const lone = only === undefined ? undefined : this.#occurrenceOf(only);
        if (
            keys.length === 1 &&
            lone?.of !== undefined &&
            this.options.onRepeatRemove !== undefined
        ) {
            this.options.onRepeatRemove(
                new CalendarRepeatRemoval(lone.of, lone.recurrenceId, this.timeZone(), this.week),
            );
            return;
        }

        // remove events that happen once, and leave repeats out of their series in one change each
        const groups = Map.groupBy(keys, (key) => this.#occurrenceOf(key).of ?? key);
        for (const [series, group] of groups) {
            if (typeof series === "string") {
                group.forEach((key) => this.#handler("onEventRemove")(this.eventOf(key)));
            } else {
                const excluded = group.reduce(
                    (event, key) =>
                        CalendarEvent.exclude(event, this.#occurrenceOf(key).recurrenceId),
                    series,
                );
                this.#handler("onEventChange")(excluded);
            }
        }
    }

    /** Read the event a drag leaves at its target: moved, turned timed or whole-day, or resized. */
    outcome(operation: DragOperation & { readonly target: DragTarget }): CalendarEvent {
        // read the event and where the drag took it from and to
        const { grab, target } = operation;
        const event = this.eventOf(operation.item);
        const zone = this.timeZone();
        const resource =
            target.kind === "time" || target.kind === "cell" ? target.resource : undefined;
        const placed = (moved: CalendarEvent) =>
            resource === undefined ? moved : { ...moved, resource };

        // move an event between days by the days between the day it was taken by and the target's
        if (grab.kind === "cell" && target.kind === "cell") {
            return placed(
                CalendarEvent.shift(
                    event,
                    PlainDate.daysUntil(grab.key, target.key) * DAY_MINUTES,
                    zone,
                ),
            );
        }
        // turn an event taken from the hours into one over whole days from the day it lands on
        else if (grab.kind === "time" && target.kind === "cell" && operation.kind === "move") {
            return placed(CalendarEvent.allDayFrom(event, target.key, zone));
        }
        // refuse every other drop but one into the hours
        else if (target.kind !== "time") {
            throw new TypeError(
                `calendar drop of '${event.key}' from a ${grab.kind} onto a ${target.kind}`,
            );
        }
        // move one edge of a timed event, a snap from the other at least
        else if (operation.kind === "resize-end" && CalendarEvent.isTimed(event)) {
            return { ...event, end: Math.max(target.at, event.start + this.snap) };
        } else if (operation.kind === "resize-start" && CalendarEvent.isTimed(event)) {
            return { ...event, start: Math.min(target.at, event.end - this.snap) };
        }
        // move a timed event by the time between where it was taken and the target
        else if (grab.kind === "time" && CalendarEvent.isTimed(event)) {
            const shift = target.at - grab.at;

            return placed({ ...event, start: event.start + shift, end: event.end + shift });
        }
        // turn an event taken from the days into one at the time it lands on
        else {
            return placed(CalendarEvent.timedAt(event, target.at));
        }
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

/** Read the nearest calendar, refusing a part outside one. */
export function useEventCalendar(): EventCalendarControl {
    const control = useContext(EventCalendarContext);
    if (control === null) {
        throw new TypeError("an event calendar part needs an event calendar around it");
    }

    return control;
}

/** The occurrences of a calendar's events in a range, by event and by key. */
interface Expansion {
    /** The range and zone the occurrences were expanded for. */
    readonly key: string;
    /** The occurrences of each event. */
    readonly byEvent: ReadonlyMap<CalendarEvent, readonly CalendarOccurrence[]>;
    /** The occurrences by their keys. */
    readonly byKey: ReadonlyMap<string, CalendarOccurrence>;
}
