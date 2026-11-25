import { state } from "@destack/theme/state";
import { plural, t } from "@destack/locale";
import { type DateSpan, nth, PlainDate, WEEK_DAYS, MonthGrid, Segment } from "@destack/schema";
import * as style from "@destack/style";
import { color, radius, size, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createMemo,
    For,
    type JSX,
    omit,
    onCleanup,
    Show,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import { Draggable, Droppable } from "../drag/index.ts";
import { mergeProperties } from "../part/index.ts";
import { EventCalendarBackground, EventCalendarEvent, placement } from "./chip.tsx";
import {
    EVENT_TYPE,
    type EventCalendarControl,
    EventCalendarDayContext,
    type EventCalendarDaysProperties,
    EventCalendarEventContext,
    type EventCalendarPartProperties,
    useEventCalendar,
    useEventCalendarDay,
    type EventCalendarView,
} from "./control.ts";
import { Paging } from "./paging.ts";
import { OfferedView } from "./view.tsx";
import { CalendarEvent } from "./event.ts";

/** The least height of a week of a month, room for a date and three events. */
const WEEK_HEIGHT = `calc(22 * ${space[1]})`;

/** The droppable of a month's days. */
const MONTH = "event-calendar-month";

/** The views the part shows. */
const MONTH_VIEWS: readonly EventCalendarView[] = ["month"];

/** A calendar as narrow as a phone. */
const NARROW_QUERY = "@container event-calendar (max-width: 36rem)";

/** The styles of a month and its days. */
const styles = style.create({
    weekNumber: {
        position: "absolute",
        insetInlineStart: space[1],
        insetBlockStart: space[1],
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
        pointerEvents: "none",
        zIndex: 1,
    },
    month: {
        display: "grid",
        flexGrow: 1,
        minHeight: 0,
        overflowY: "auto",
        gridTemplateRows: "auto",
        gridAutoRows: `minmax(${WEEK_HEIGHT}, 1fr)`,
        borderBlockWidth: stroke.border,
        borderInlineWidth: { default: stroke.border, [NARROW_QUERY]: 0 },
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: { default: radius[4], [NARROW_QUERY]: 0 },
    },
    weekdays: {
        display: "grid",
        borderBlockEndWidth: stroke.border,
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
    },
    weekday: {
        paddingBlock: space[1],
        textAlign: "center",
        color: color.mutedForeground,
    },
    weekdayName: {
        textDecoration: "none",
    },
    week: {
        position: "relative",
        display: "grid",
        rowGap: space[1],
        paddingBlockEnd: space[1],
        borderBlockEndWidth: { default: stroke.border, ":last-child": 0 },
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
    },
    cell: {
        display: "contents",
    },
    day: {
        // reach over the space below the events to the next week
        gridRow: "1 / -1",
        marginBlockEnd: `calc(-1 * ${space[1]})`,
    },
    divided: {
        borderInlineStartWidth: stroke.border,
        borderInlineStartStyle: "solid",
        borderInlineStartColor: color.border,
    },
    weekend: {
        backgroundColor: `color-mix(in oklab, ${color.muted} 50%, transparent)`,
    },
    number: {
        gridRow: 1,
        justifySelf: "end",
        alignSelf: "start",
        marginInline: space[1],
        marginBlockStart: space[1],
        minWidth: size[1],
        borderRadius: radius.full,
        textAlign: "center",
        zIndex: 1,
    },
    outside: {
        color: color.mutedForeground,
    },
    today: {
        fontWeight: weight.semibold,
    },
    more: {
        justifySelf: "start",
        marginInline: { default: space[1], [NARROW_QUERY]: 0 },
        paddingInline: space[1],
        borderRadius: radius[2],
        color: color.mutedForeground,
        fontWeight: weight.medium,
        zIndex: 1,
    },
});

/** The places of a month's days and events in a week's grid. */
const places = style.create({
    columns: (days: number) => ({ gridTemplateColumns: `repeat(${String(days)}, minmax(0, 1fr))` }),
    span: (column: number, span: number, row: number) => ({
        gridColumn: `${String(column)} / span ${String(span)}`,
        gridRow: String(row),
    }),
    column: (column: number) => ({ gridColumn: String(column) }),
    shade: (column: number, span: number) => ({
        gridColumn: `${String(column)} / span ${String(span)}`,
        gridRow: "1 / -1",
    }),
    rows: (events: number) => ({
        gridTemplateRows: `repeat(${String(events + 2)}, auto) 1fr`,
    }),
});

/** The month a calendar shows: its grid of weeks, its events and the days a drag measures. */
export class EventCalendarMonthControl {
    /** The calendar. */
    readonly control: EventCalendarControl;
    /** The six weeks of the month from the locale's first day. */
    readonly grid: Accessor<MonthGrid>;
    /** The weeks' shown days, the hidden weekdays left out. */
    readonly weeks: Accessor<PlainDate[][]>;
    /** The days' spans of the events shown over them. */
    readonly spans: Accessor<DateSpan[]>;
    /** The days' spans of the whole-day events shaded behind them. */
    readonly shades: Accessor<DateSpan[]>;
    /** The elements of the days, which a drag measures. */
    readonly cells = new Map<PlainDate, HTMLElement>();

    /** Lay out the calendar's month, anew only when the month changes. */
    constructor(control: EventCalendarControl) {
        // lay the month out in six weeks from the locale's first day
        this.control = control;
        const first = createMemo(() => PlainDate.with(control.date(), { day: 1 }));
        this.grid = createMemo(() => MonthGrid.of(first(), control.week, "fixed"));
        this.weeks = createMemo(() =>
            this.grid().weeks.map((week) => week.filter((day) => !control.isHidden(day))),
        );

        // read the events shown over the days, and the whole days shaded behind them
        this.spans = createMemo(() =>
            control
                .events()
                .filter((event) => !CalendarEvent.isBackground(event))
                .map((event) => CalendarEvent.span(event, control.timeZone())),
        );
        this.shades = createMemo(() =>
            control
                .events()
                .filter(
                    (event) => CalendarEvent.isAllDay(event) && CalendarEvent.isBackground(event),
                )
                .map((event) => CalendarEvent.span(event, control.timeZone())),
        );
    }

    /** Report whether a drag aims at a day. */
    isTarget(day: PlainDate): boolean {
        const target = this.control.manager.targetIn(this.control.dragId(MONTH));

        return target?.kind === "cell" && target.key === day;
    }
}

/** The month the month's parts render. */
const EventCalendarMonthContext = createContext<EventCalendarMonthControl | null>(null);

/** Read the month a part of the month view renders. */
export function useEventCalendarMonth(): EventCalendarMonthControl {
    const month = useContext(EventCalendarMonthContext);
    if (month === null) {
        throw new TypeError("a calendar month part needs a calendar month around it");
    }

    return month;
}

/** A week of a month and its events stacked over its days. */
export interface CalendarWeek {
    /** The week's shown days. */
    readonly days: readonly PlainDate[];
    /** The week's events, stacked in rows. */
    readonly segments: Accessor<Segment<PlainDate>[]>;
}

/** The week the week's parts render. */
const EventCalendarWeekContext = createContext<CalendarWeek | null>(null);

/** Read the week a part of a month's week renders. */
export function useEventCalendarWeek(): CalendarWeek {
    const week = useContext(EventCalendarWeekContext);
    if (week === null) {
        throw new TypeError("a calendar week part needs a calendar week around it");
    }

    return week;
}

/** Render the month of the calendar's date in whole weeks: weekday names and weeks by default. */
export function EventCalendarMonth(properties: EventCalendarPartProperties): JSX.Element {
    return <OfferedView views={MONTH_VIEWS}>{() => <MonthView {...properties} />}</OfferedView>;
}

/** Render the month view while it shows. */
function MonthView(properties: EventCalendarPartProperties): JSX.Element {
    // lay the month out, and move the focused day through it by the keys
    const control = useEventCalendar();
    const locale = useLocale();
    const month = new EventCalendarMonthControl(control);
    const measured = new Map<PlainDate, DOMRect>();
    const rest = omit(properties, "children", "xstyle", "style");
    let element: HTMLElement | undefined;

    // take dropped events on the days, stepping a day along a week and a week across weeks
    void new Droppable(
        {
            id: control.dragId(MONTH),
            group: control.dragId(MONTH),
            axis: "both",
            element: () => element,
            accepts: (draggable) => control.accepts(draggable, "cell"),
            measure: () => {
                measured.clear();
                for (const [key, cell] of month.cells) {
                    measured.set(key, cell.getBoundingClientRect());
                }
            },
            targetAt: (x, y) => {
                const found = [...measured].find(
                    ([, bounds]) =>
                        x >= bounds.left &&
                        x <= bounds.right &&
                        y >= bounds.top &&
                        y <= bounds.bottom,
                );

                return found === undefined
                    ? undefined
                    : { kind: "cell", droppable: control.dragId(MONTH), key: found[0] };
            },
            step: (target, step) => {
                // move a day along the row in the text's direction, or a week across rows
                const offsets = {
                    "inline-start": -1,
                    "inline-end": 1,
                    "block-start": -WEEK_DAYS,
                    "block-end": WEEK_DAYS,
                };
                const date = target.kind === "cell" ? target.key : undefined;

                return date === undefined
                    ? undefined
                    : {
                          kind: "cell",
                          droppable: control.dragId(MONTH),
                          key: control.skipHidden(
                              PlainDate.add(date, { days: offsets[step] }),
                              offsets[step] < 0 ? -1 : 1,
                          ),
                      };
            },
            enter: (from) => from,
            describe: (target) => {
                // name a day by its full date
                if (target.kind !== "cell") {
                    throw new TypeError(`calendar day described as a ${target.kind} target`);
                }

                return {
                    kind: "label",
                    label: locale.plainDate(target.key, { dateStyle: "full" }),
                };
            },
            drop: (operation) => control.drop(operation),
        },
        control.manager,
    );

    // turn the pages by a finger's swipe
    const paging = new Paging(control, () => locale.direction === "rtl");

    return (
        <EventCalendarMonthContext value={month}>
            <div
                data-slot="event-calendar-month"
                role="grid"
                {...mergeProperties(
                    {
                        ...paging.attributes(),
                        ref: (created: HTMLDivElement) => (element = created),
                        onKeyDown: (event: KeyboardEvent) =>
                            event.defaultPrevented
                                ? undefined
                                : control.focus.move(event, locale.direction),
                        onFocusOut: (event: FocusEvent & { readonly currentTarget: Element }) =>
                            control.focus.focusOut(event),
                    },
                    rest,
                )}
                {...style.attributes(
                    [styles.month, paging.style(), properties.xstyle],
                    properties.style,
                )}
            >
                {properties.children ?? (
                    <>
                        <EventCalendarWeekdayHeader />
                        <EventCalendarDayGrid />
                    </>
                )}
            </div>
        </EventCalendarMonthContext>
    );
}

/** Render the row of weekday names above the weeks. */
export function EventCalendarWeekdayHeader(properties: EventCalendarPartProperties): JSX.Element {
    // name the weekdays of the first week, the hidden ones left out
    const locale = useLocale();
    const month = useEventCalendarMonth();
    const rest = omit(properties, "children", "xstyle", "style");

    return (
        <div
            role="row"
            data-slot="event-calendar-weekday-header"
            {...rest}
            {...style.attributes(
                [styles.weekdays, places.columns(nth(month.weeks(), 0).length), properties.xstyle],
                properties.style,
            )}
        >
            <For each={month.weeks()[0]}>
                {(day) => (
                    <div role="columnheader" {...style.attrs(text.xs, styles.weekday)}>
                        <abbr
                            title={locale.plainDate(day, { weekday: "long" })}
                            {...style.attrs(styles.weekdayName)}
                        >
                            {locale.plainDate(day, { weekday: "short" })}
                        </abbr>
                    </div>
                )}
            </For>
        </div>
    );
}

/** The properties of the list of a month's weeks. */
export interface EventCalendarDayGridProperties {
    /** Render a week's part, the default week when absent. */
    readonly children?: (week: CalendarWeek) => JSX.Element;
}

/** Render the month's weeks, each from the function given or as the default week. */
export function EventCalendarDayGrid(properties: EventCalendarDayGridProperties): JSX.Element {
    const control = useEventCalendar();
    const month = useEventCalendarMonth();

    return (
        <For each={month.weeks()}>
            {(days) => {
                // stack the events over the week's days
                const segments = createMemo(() => control.stack(month.spans(), days));
                const week = { days, segments };

                return (
                    <EventCalendarWeekContext value={week}>
                        {properties.children === undefined ? (
                            <EventCalendarWeek />
                        ) : (
                            properties.children(week)
                        )}
                    </EventCalendarWeekContext>
                );
            }}
        </For>
    );
}

/** Render a week of a month: its days with their events, and whole days shaded behind them. */
export function EventCalendarWeek(
    properties: EventCalendarDaysProperties & Omit<EventCalendarPartProperties, "children">,
): JSX.Element {
    // lay the week's days out in columns, with rows for the event limit and the link past it
    const control = useEventCalendar();
    const locale = useLocale();
    const month = useEventCalendarMonth();
    const { days } = useEventCalendarWeek();
    const rest = omit(properties, "children", "xstyle", "style");

    return (
        <div
            role="row"
            data-slot="event-calendar-week"
            {...rest}
            {...style.attributes(
                [
                    styles.week,
                    places.columns(days.length),
                    places.rows(control.dayMaxEvents()),
                    properties.xstyle,
                ],
                properties.style,
            )}
        >
            {/* Days */}
            <For each={days}>
                {(day, index) => (
                    <EventCalendarDayContext value={{ day, index }}>
                        {properties.children === undefined ? (
                            <EventCalendarMonthDay />
                        ) : (
                            properties.children({ day, index })
                        )}
                    </EventCalendarDayContext>
                )}
            </For>

            {/* The week of the year, at the row's start */}
            <Show when={control.options.weekNumbers === true}>
                <span
                    data-slot="event-calendar-week-number"
                    aria-hidden="true"
                    {...style.attrs(text.xs, styles.weekNumber)}
                >
                    {locale.render(control.weekNumber(nth(days, 0)))}
                </span>
            </Show>

            {/* Whole days shaded behind the events */}
            <For each={control.stack(month.shades(), days)}>
                {(segment) => {
                    const columns = () => Segment.columns(segment, days);

                    return (
                        <EventCalendarBackground
                            aria-hidden="true"
                            event={control.eventOf(segment.key)}
                            xstyle={places.shade(columns().start + 1, columns().span)}
                        />
                    );
                }}
            </For>
        </div>
    );
}

/** Render a day of a month: its date, the events that start on it and a link to the rest. */
export function EventCalendarMonthDay(properties: EventCalendarPartProperties): JSX.Element {
    // mark the day a drag aims at, and measure it for drags
    const control = useEventCalendar();
    const month = useEventCalendarMonth();
    const { day, index } = useEventCalendarDay();
    const rest = omit(properties, "children", "xstyle", "style");
    let cell: HTMLDivElement | undefined;
    onCleanup(() => {
        // forget the cell unless a new grid's cell for the day took its place first
        if (month.cells.get(day) === cell) {
            month.cells.delete(day);
        }
    });

    return (
        <div
            role="gridcell"
            aria-selected={PlainDate.equals(day, control.date()) ? "true" : "false"}
            {...style.attrs(styles.cell)}
        >
            {/* Background */}
            <div
                data-slot="event-calendar-day"
                data-date={day}
                data-target={month.isTarget(day) ? "" : undefined}
                {...mergeProperties(
                    {
                        ref: (created: HTMLDivElement) => {
                            cell = created;
                            month.cells.set(day, created);
                        },
                    },
                    rest,
                )}
                {...style.attributes(
                    [
                        state.focusInset,
                        styles.day,
                        index() > 0 && styles.divided,
                        places.column(index() + 1),
                        control.isRestDay(day) && styles.weekend,
                        month.isTarget(day) && state.target,
                        properties.xstyle,
                    ],
                    properties.style,
                )}
            />
            {properties.children ?? (
                <>
                    <EventCalendarDate />
                    <EventCalendarDayContent />
                    <EventCalendarMore />
                </>
            )}
        </div>
    );
}

/** Render a day's date, the month's tab stop, opening the day on a press. */
export function EventCalendarDate(
    properties: Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "class"> & {
        /** The StyleX styles applied after the date's. */
        readonly xstyle?: style.Styles;
    },
): JSX.Element {
    // focus the day's date as the month's tab stop
    const control = useEventCalendar();
    const locale = useLocale();
    const month = useEventCalendarMonth();
    const { day, index } = useEventCalendarDay();
    const rest = omit(properties, "children", "xstyle", "style");
    const isToday = () => PlainDate.equals(day, control.today());
    let date: HTMLElement | undefined;
    control.focus.bind(
        () => day,
        () => date,
    );

    return (
        <button
            type="button"
            data-slot="event-calendar-date"
            data-today={isToday() ? "" : undefined}
            tabindex={control.focus.isActive(day) ? 0 : -1}
            aria-label={locale.plainDate(day, { dateStyle: "full" })}
            {...mergeProperties(
                {
                    ref: (created: HTMLButtonElement) => (date = created),
                    onFocus: () => control.focus.focusIn(day),
                    onClick: () => control.open(day),
                },
                rest,
            )}
            {...style.attributes(
                [
                    text.xs,
                    state.hover,
                    styles.number,
                    places.column(index() + 1),
                    MonthGrid.isOutside(month.grid(), day) && styles.outside,
                    isToday() && [state.checked, styles.today],
                    properties.xstyle,
                ],
                properties.style,
            )}
        >
            {properties.children ?? locale.number(PlainDate.day(day))}
        </button>
    );
}

/** Render the events that start on a day, up to the calendar's limit. */
export function EventCalendarDayContent(): JSX.Element {
    // read the week's stacked events that start on the day
    const control = useEventCalendar();
    const { day } = useEventCalendarDay();
    const { days, segments } = useEventCalendarWeek();

    return (
        <For
            each={segments().filter(
                (segment) => segment.start === day && segment.position < control.dayMaxEvents(),
            )}
            keyed={(segment) => control.segmentKey(segment)}
        >
            {(segment) => <MonthEvent segment={segment()} days={days} />}
        </For>
    );
}

/** Render the link to a day's events past the limit, opening the day. */
export function EventCalendarMore(
    properties: Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "class"> & {
        /** The StyleX styles applied after the link's. */
        readonly xstyle?: style.Styles;
    },
): JSX.Element {
    // count the events of the day the limit leaves out
    const control = useEventCalendar();
    const locale = useLocale();
    const { day, index } = useEventCalendarDay();
    const { segments } = useEventCalendarWeek();
    const rest = omit(properties, "children", "xstyle", "style");
    const hidden = () => Segment.hidden(segments(), day, control.dayMaxEvents());

    return (
        <Show when={hidden() > 0}>
            <button
                type="button"
                data-slot="event-calendar-more"
                {...mergeProperties({ onClick: () => control.open(day) }, rest)}
                {...style.attributes(
                    [
                        text.xs,
                        state.hover,
                        styles.more,
                        places.span(index() + 1, 1, control.dayMaxEvents() + 2),
                        properties.xstyle,
                    ],
                    properties.style,
                )}
            >
                {properties.children ??
                    locale.render(t`${plural(hidden(), { one: "+# more", other: "+# more" })}`)}
            </button>
        </Show>
    );
}

/** Render an event's segment in a week's row, lifted to other days by pointer, touch and Space. */
function MonthEvent(properties: {
    /** The segment of the event in the row. */
    readonly segment: Segment<PlainDate>;
    /** The shown days of the row. */
    readonly days: readonly PlainDate[];
}): JSX.Element {
    // drag the event from this part's first day to another
    const control = useEventCalendar();
    const Render = useContext(EventCalendarEventContext) ?? EventCalendarEvent;
    const event = () => control.eventOf(properties.segment.key);
    let element: HTMLElement | undefined;
    const lifted = new Draggable(
        {
            id: control.dragId(
                untrack(() => JSON.stringify([properties.segment.key, properties.segment.start])),
            ),
            item: untrack(() => properties.segment.key),
            type: EVENT_TYPE,
            kind: "move",
            label: () => event().title,
            element: () => element,
            home: () => ({
                kind: "cell",
                droppable: control.dragId(MONTH),
                key: properties.segment.start,
            }),
        },
        control.manager,
    );
    const draggable = untrack(() => control.isBusy(event())) ? {} : lifted.attributes([" "]);
    const columns = () => Segment.columns(properties.segment, properties.days);

    // forward the drag and the element to the event the caller renders
    const eventAttributes = mergeProperties(
        { ref: (created: HTMLButtonElement) => (element = created) },
        draggable,
    );

    return (
        <Render
            event={event()}
            layout="inline"
            isStart={properties.segment.isStart}
            isEnd={properties.segment.isEnd}
            data-start={properties.segment.start}
            data-end={properties.segment.end}
            {...eventAttributes}
            xstyle={[
                places.span(columns().start + 1, columns().span, properties.segment.position + 2),
                control.manager.isDragging(properties.segment.key) && placement.dragging,
            ]}
        />
    );
}
