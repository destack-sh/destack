import { state } from "@destack/theme/state";
import { t } from "@destack/locale";
import { nth, PlainDate, Week, Segment, TimeScale, type TimeSpan } from "@destack/schema";
import * as style from "@destack/style";
import { color, radius, size, space, stroke, weight, swatch } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    createMemo,
    createSignal,
    For,
    type JSX,
    Show,
    useEnvironment,
    untrack,
    useLocale,
    Dynamic,
    useContext,
} from "@destack/view";
import { createResizeObserver } from "@destack/view/primitives/resize-observer";
import { Draggable, type DragStep, type DragTarget, Droppable } from "../drag/index.ts";
import { Move } from "../interaction/index.ts";
import { mergeProperties } from "../part/index.ts";
import { EventCalendarBackground, EventCalendarEvent } from "./event.tsx";
import {
    COLUMNS,
    columnsOf,
    EVENT_TYPE,
    isAllDay,
    isBackground,
    isTimed,
    type TimedCalendarEvent,
    useEventCalendar,
    EventCalendarEventContext,
} from "./control.ts";

/** The height of an hour in the time grid, as Google Calendar's default zoom shows about twelve. */
const HOUR_HEIGHT = "3rem";

/** The width of the hour labels beside the day columns. */
const GUTTER = "4rem";

/** The hour week and day views open scrolled to, the start of a working day. */
const OPENING_HOUR = 7;

/** The share of an hour shown above the opening hour, so its label is whole. */
const OPENING_LEAD = 0.25;

/** The hours a day shows, the height of a day of twenty-four. */
const DAY_HOURS = 24;

/** The least height of a row of all-day events, a line of extra-small text and its padding. */
const ALL_DAY_ROW = "1.5rem";

/** The droppable of the all-day row's days. */
const DAYS = "event-calendar-days";

/** The styles of a time grid. */
const styles = style.create({
    grid: {
        display: "flex",
        flexDirection: "column",
        minHeight: 0,
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[4],
        overflow: "hidden",
    },
    row: {
        display: "grid",
        borderBlockEndWidth: stroke.border,
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
    },
    heading: {
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        paddingBlock: space[1],
        color: color.mutedForeground,
    },
    date: {
        minWidth: size[2],
        borderRadius: radius.full,
        color: color.foreground,
        fontWeight: weight.semibold,
    },
    allDay: {
        gridAutoRows: `minmax(${ALL_DAY_ROW}, auto)`,
        rowGap: space[1],
        paddingBlock: space[1],
    },
    scroller: {
        flexGrow: 1,
        minHeight: 0,
        overflowY: "auto",
    },
    body: {
        position: "relative",
        display: "grid",
    },
    hours: {
        position: "relative",
    },
    hour: {
        position: "absolute",
        insetInlineEnd: space[2],
        transform: "translateY(-50%)",
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
    },
    column: {
        position: "relative",
        borderInlineStartWidth: stroke.border,
        borderInlineStartStyle: "solid",
        borderInlineStartColor: color.border,
        cursor: "cell",
    },
    weekend: {
        backgroundColor: `color-mix(in oklab, ${color.muted} 50%, transparent)`,
    },
    line: {
        position: "absolute",
        insetInline: 0,
        borderBlockStartWidth: stroke.border,
        borderBlockStartStyle: "solid",
        borderBlockStartColor: `color-mix(in oklab, ${color.border} 60%, transparent)`,
    },
    event: {
        position: "absolute",
        minHeight: `calc(${space[4]} + ${space[1]})`,
        paddingInlineEnd: space[1],
    },
    fill: {
        width: "100%",
        height: "100%",
    },
    handle: {
        position: "absolute",
        insetInline: 0,
        insetBlockEnd: 0,
        height: space[2],
        cursor: "ns-resize",
        zIndex: 2,
    },
    now: {
        position: "absolute",
        insetInline: 0,
        borderBlockStartWidth: stroke.ring,
        borderBlockStartStyle: "solid",
        borderBlockStartColor: swatch.destructiveMark,
        zIndex: 3,
        pointerEvents: "none",
    },
    shade: {
        position: "absolute",
    },
    sweep: {
        position: "absolute",
        insetInline: space[1],
        borderRadius: radius[2],
        backgroundColor: `color-mix(in oklab, ${swatch.accentMark} 20%, transparent)`,
        pointerEvents: "none",
    },
});

/** The places of the time grid's parts. */
const places = style.create({
    hours: (hourHeight: string) => ({ height: `calc(${hourHeight} * ${String(DAY_HOURS)})` }),
    columns: (days: number) => ({
        gridTemplateColumns: `${GUTTER} repeat(${String(days)}, minmax(0, 1fr))`,
    }),
    column: (column: number) => ({ gridColumn: String(column) }),
    span: (column: number, span: number, row: number) => ({
        gridColumn: `${String(column)} / span ${String(span)}`,
        gridRow: String(row),
    }),
    shade: (column: number, span: number, rows: number) => ({
        gridColumn: `${String(column)} / span ${String(span)}`,
        gridRow: `1 / span ${String(rows)}`,
    }),
    at: (top: number) => ({ top: `${String(top * 100)}%` }),
    box: (top: number, height: number, left: number, width: number) => ({
        top: `${String(top * 100)}%`,
        height: `${String(height * 100)}%`,
        insetInlineStart: `${String(left * 100)}%`,
        width: `${String(width * 100)}%`,
    }),
});

/** Render the days of a week or a single day in hours: all-day events on top, timed ones below. */
export function EventCalendarTime(): JSX.Element {
    // read the shown days and split the events into whole days and times within a day
    const control = useEventCalendar();
    const locale = useLocale();
    const days = createMemo(() => control.days());
    const allDay = createMemo(() =>
        control
            .events()
            .filter((event) => isAllDay(event) && !isBackground(event))
            .map((event) => control.spanOf(event)),
    );
    const shades = createMemo(() =>
        control.events().filter((event) => isAllDay(event) && isBackground(event)),
    );
    const timed = createMemo(() =>
        control
            .events()
            .filter((event): event is TimedCalendarEvent => isTimed(event) && !isAllDay(event)),
    );
    const stacked = createMemo(() => control.stack(allDay(), days()));
    const levels = () => Math.max(1, ...stacked().map((each) => each.positions));

    // take all-day events on the row of days, a day at a time
    let row: HTMLElement | undefined;
    let dayColumns: DOMRect[] = [];
    void new Droppable(
        {
            id: DAYS,
            group: DAYS,
            axis: "inline",
            element: () => row,
            accepts: () => true,
            measure: () => {
                // measure each day's column, which the all-day row shares in either direction
                const grid = row?.closest("[data-slot=event-calendar-time]");
                if (grid === null || grid === undefined) {
                    throw new TypeError("calendar all-day row measured outside its grid");
                }
                const elements = grid.querySelectorAll("[data-slot=event-calendar-column]");
                dayColumns = [...elements].map((element) => element.getBoundingClientRect());
            },
            targetAt: (x) => {
                // take the day whose column is under the pointer
                const index = dayColumns.findIndex(
                    (bounds) => x >= bounds.left && x < bounds.right,
                );
                const day = days()[index];

                return day === undefined ? undefined : { kind: "cell", droppable: DAYS, key: day };
            },
            step: (target, step) => stepDay(target, step, days()),
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
        },
        control.manager,
    );

    // open on the working day's start once the hours have a height, as Google Calendar opens its week
    const [scroller, setScroller] = createSignal<HTMLElement>();
    let isOpened = false;
    createResizeObserver(scroller, (_rectangle, element) => {
        if (isOpened || element.scrollHeight === 0) {
            return;
        }
        isOpened = true;
        const hour = control.options.openingHour ?? OPENING_HOUR;
        element.scrollTop = (element.scrollHeight * (hour - OPENING_LEAD)) / DAY_HOURS;
    });

    return (
        <div data-slot="event-calendar-time" {...style.attrs(styles.grid)}>
            {/* Day headings */}
            <div {...style.attrs(styles.row, places.columns(days().length))}>
                <For each={days()}>
                    {(day, column) => (
                        <button
                            type="button"
                            data-slot="event-calendar-heading"
                            onClick={() => {
                                control.setDate(day);
                                control.setView("day");
                            }}
                            {...style.attrs(text.xs, styles.heading, places.column(column() + 2))}
                        >
                            {locale.plainDate(day, { weekday: "short" })}
                            <span
                                {...style.attrs(
                                    text.lg,
                                    styles.date,
                                    PlainDate.equals(day, control.today()) && state.checked,
                                )}
                            >
                                {locale.number(PlainDate.day(day))}
                            </span>
                        </button>
                    )}
                </For>
            </div>

            {/* All-day events */}
            <div
                data-slot="event-calendar-all-day"
                ref={(created) => (row = created)}
                {...style.attrs(styles.row, styles.allDay, places.columns(days().length))}
            >
                <For
                    each={control.stack(
                        shades().map((event) => control.spanOf(event)),
                        days(),
                    )}
                >
                    {(segment) => {
                        const columns = () => columnsOf(segment, days());

                        return (
                            <EventCalendarBackground
                                event={control.eventOf(segment.key)}
                                xstyle={places.shade(columns().start + 2, columns().span, levels())}
                            />
                        );
                    }}
                </For>
                <For each={stacked()}>
                    {(segment) => <AllDayEvent segment={segment} days={days()} />}
                </For>
            </div>

            {/* Hours */}
            <div
                data-slot="event-calendar-scroller"
                ref={setScroller}
                {...style.attrs(styles.scroller)}
            >
                <div
                    {...style.attrs(
                        styles.body,
                        places.hours(control.options.hourHeight ?? HOUR_HEIGHT),
                        places.columns(days().length),
                    )}
                >
                    <HourLabels />
                    <For each={days()}>
                        {(day, column) => (
                            <DayColumn day={day} column={column() + 2} events={timed()} />
                        )}
                    </For>
                </div>
            </div>
        </div>
    );
}

/** Render the local hours of the first shown day beside the columns. */
function HourLabels(): JSX.Element {
    // mark each local hour of the day, a daylight-saving day showing 23 or 25
    const control = useEventCalendar();
    const locale = useLocale();
    const scale = () => control.dayScale(nth(control.days(), 0));
    const ticks = () => control.hours(scale());

    return (
        <div aria-hidden="true" {...style.attrs(styles.hours)}>
            <For each={ticks()}>
                {(tick) => (
                    <span
                        {...style.attrs(
                            text.xs,
                            styles.hour,
                            places.at(TimeScale.at(scale(), tick.at)),
                        )}
                    >
                        {locale.date(tick.at, { hour: "numeric", timeZone: control.timeZone() })}
                    </span>
                )}
            </For>
        </div>
    );
}

/** Render a day's column of hours: lines, now, timed events, and a sweep to create one. */
function DayColumn(properties: {
    /** The day. */
    readonly day: PlainDate;
    /** The grid column, from 1. */
    readonly column: number;
    /** The timed events within a day. */
    readonly events: readonly TimedCalendarEvent[];
}): JSX.Element {
    // read the day's span of time and its events cut and packed side by side
    const control = useEventCalendar();
    const locale = useLocale();
    const scale = () => control.dayScale(properties.day);
    const spans = (background: boolean): TimeSpan[] =>
        properties.events
            .filter((event) => isBackground(event) === background)
            .map((event) => ({ key: event.key, start: event.start, end: event.end }));
    const segments = createMemo(() => Segment.pack(spans(false), scale().start, scale().end));
    const shades = createMemo(() =>
        spans(true).filter((span) => span.end > scale().start && span.start < scale().end),
    );
    const ticks = () => control.hours(scale());
    const id = control.column(properties.day);

    // read the span a dragged event would take in this column
    const landing = (): TimeSpan | undefined => {
        // leave drags that aim elsewhere alone
        const operation = control.manager.operation();
        const target = operation?.target;
        if (operation === undefined || target?.kind !== "time" || target.droppable !== id) {
            return undefined;
        }

        // move its end alone, or the whole event by the time between its grab and the target
        const isEnd = operation.kind === "resize-end";
        const key = operation.item;
        const event = control.timedOf(key);
        const grab = operation.grab;
        if (grab.kind !== "time") {
            throw new TypeError(`calendar time drag taken from a ${grab.kind} target: '${key}'`);
        }
        const shift = target.at - grab.at;

        return isEnd
            ? { key, start: event.start, end: Math.max(target.at, event.start + control.snap) }
            : { key, start: event.start + shift, end: event.end + shift };
    };
    let element: HTMLElement | undefined;
    let bounds: DOMRect | undefined;

    // take timed events at a snapped time, moved by the pointer's travel or a snap per arrow key
    void new Droppable(
        {
            id,
            group: COLUMNS,
            axis: "block",
            element: () => element,
            accepts: () => true,
            measure: () => {
                bounds = element?.getBoundingClientRect();
            },
            targetAt: (_x, _y, operation) => {
                // read where the dragged edge stood as a share of its day, and move it by the travel
                const isEnd = operation.kind === "resize-end";
                const key = operation.item;
                const event = control.eventOf(key);
                if (!isTimed(event) || bounds === undefined) {
                    return undefined;
                }
                const edge = isEnd ? event.end : event.start;
                const own = control.dayScale(isEnd ? control.dayOf(event.start) : properties.day);
                const share =
                    TimeScale.at(control.dayScale(control.dayOf(event.start)), edge) +
                    operation.offsetY / bounds.height;
                const at = snap(TimeScale.invert(own, share), own.start, control.snap);

                return {
                    kind: "time",
                    droppable: isEnd ? control.column(control.dayOf(event.start)) : id,
                    at,
                };
            },
            step: (target, step) => {
                // step a snap up or down within the day
                const at = target.kind === "time" ? target.at : scale().start;
                const next =
                    step === "block-end"
                        ? at + control.snap
                        : step === "block-start"
                          ? at - control.snap
                          : undefined;

                return next === undefined || next < scale().start || next >= scale().end
                    ? undefined
                    : { kind: "time", droppable: id, at: next };
            },
            enter: (from) => {
                // take the same local time on this day
                const share =
                    from.kind === "time"
                        ? TimeScale.at(control.dayScale(control.dayOf(from.at)), from.at)
                        : 0;

                return {
                    kind: "time",
                    droppable: id,
                    at: snap(TimeScale.invert(scale(), share), scale().start, control.snap),
                };
            },
            describe: (target) => {
                // name a time by its weekday, hour and minute in the calendar's zone
                if (target.kind !== "time") {
                    throw new TypeError(`calendar time described as a ${target.kind} target`);
                }
                const label = locale.date(target.at, {
                    weekday: "long",
                    hour: "numeric",
                    minute: "2-digit",
                    timeZone: control.timeZone(),
                });

                return { kind: "label", label };
            },
        },
        control.manager,
    );

    // sweep out a new event over empty time, snapped
    const [sweep, setSweep] = createSignal<TimeScale | undefined>(undefined);
    const atPointer = (y: number): number => {
        const box = element?.getBoundingClientRect();
        const share = box === undefined ? 0 : (y - box.top) / box.height;

        return snap(TimeScale.invert(scale(), share), scale().start, control.snap);
    };
    let anchor = 0;
    const move = new Move(
        {
            canStart: (event) => event.target === element && control.options.onCreate !== undefined,
            onMoveStart: (event) => {
                // anchor the sweep where the pointer pressed
                anchor = atPointer(event.clientY - event.offsetY);
                setSweep({ start: anchor, end: anchor + control.snap });
            },
            onMove: (event) => {
                const at = atPointer(event.clientY);
                setSweep({ start: Math.min(anchor, at), end: Math.max(anchor + control.snap, at) });
            },
            onMoveEnd: (event) => {
                const swept = sweep();
                setSweep(undefined);
                if (swept !== undefined && !event.isCancelled) {
                    control.options.onCreate?.(swept);
                }
            },
        },
        useEnvironment().clock,
    );

    return (
        <div
            data-slot="event-calendar-column"
            data-date={properties.day}
            {...mergeProperties(
                { ref: (created: HTMLDivElement) => (element = created) },
                move.attributes(),
            )}
            {...style.attrs(
                styles.column,
                places.column(properties.column),
                Week.isWeekend(control.week, properties.day) && styles.weekend,
            )}
        >
            {/* Hour lines */}
            <For each={ticks()}>
                {(tick) => (
                    <div
                        aria-hidden="true"
                        {...style.attrs(styles.line, places.at(TimeScale.at(scale(), tick.at)))}
                    />
                )}
            </For>

            {/* Events shown as shading, cut to the day */}
            <For each={shades()}>
                {(span) => {
                    const start = () => Math.max(span.start, scale().start);
                    const end = () => Math.min(span.end, scale().end);

                    return (
                        <EventCalendarBackground
                            event={control.eventOf(span.key)}
                            xstyle={[
                                styles.shade,
                                places.box(
                                    TimeScale.at(scale(), start()),
                                    (end() - start()) / (scale().end - scale().start),
                                    0,
                                    1,
                                ),
                            ]}
                        />
                    );
                }}
            </For>

            {/* Events */}
            <For each={segments()}>
                {(segment) => (
                    <TimedEvent
                        event={control.timedOf(segment.key)}
                        segment={segment}
                        scale={scale()}
                        column={id}
                    />
                )}
            </For>

            {/* The sweep of a new event */}
            <Show when={sweep()}>
                {(swept) => (
                    <div
                        data-slot="event-calendar-sweep"
                        {...style.attrs(
                            styles.sweep,
                            places.box(
                                TimeScale.at(scale(), swept().start),
                                (swept().end - swept().start) / (scale().end - scale().start),
                                0,
                                1,
                            ),
                        )}
                    />
                )}
            </Show>

            {/* Where a dragged event would land */}
            <Show when={landing()}>
                {(landed) => (
                    <div
                        data-slot="event-calendar-landing"
                        aria-hidden="true"
                        {...style.attrs(
                            styles.sweep,
                            places.box(
                                TimeScale.at(scale(), landed().start),
                                (landed().end - landed().start) / (scale().end - scale().start),
                                0,
                                1,
                            ),
                        )}
                    />
                )}
            </Show>

            {/* The current time on today's column */}
            <Show
                when={PlainDate.equals(properties.day, control.today()) ? control.now() : undefined}
            >
                {(now) => (
                    <div
                        data-slot="event-calendar-now"
                        aria-hidden="true"
                        {...style.attrs(styles.now, places.at(TimeScale.at(scale(), now())))}
                    />
                )}
            </Show>
        </div>
    );
}

/** Render a timed event in its day's column, moved from anywhere and resized from its end. */
function TimedEvent(properties: {
    /** The event. */
    readonly event: TimedCalendarEvent;
    /** Its segment in the column. */
    readonly segment: Segment<number>;
    /** The column's span of time. */
    readonly scale: TimeScale;
    /** The column's droppable. */
    readonly column: string;
}): JSX.Element {
    // drag the event's start, or its end from the handle at its foot
    const control = useEventCalendar();
    const render = useContext(EventCalendarEventContext) ?? EventCalendarEvent;
    const locale = useLocale();
    const key = untrack(() => properties.event.key);
    let element: HTMLElement | undefined;
    let handle: HTMLElement | undefined;
    const start = new Draggable(
        {
            id: JSON.stringify([key, untrack(() => properties.segment.start)]),
            item: key,
            type: EVENT_TYPE,
            kind: "move",
            label: () => properties.event.title,
            element: () => element,
            home: () => ({
                kind: "time",
                droppable: properties.column,
                at: properties.segment.start,
            }),
        },
        control.manager,
    );
    const move = start.attributes([" "]);

    // stretch the event's end from the part that shows it, the one part with an end handle
    const end = untrack(() => properties.segment.isEnd)
        ? new Draggable(
              {
                  id: JSON.stringify([key, "end"]),
                  item: key,
                  type: EVENT_TYPE,
                  kind: "resize-end",
                  label: () => properties.event.title,
                  element: () => handle,
                  home: () => ({
                      kind: "time",
                      droppable: properties.column,
                      at: properties.event.end,
                  }),
              },
              control.manager,
          )
        : undefined;
    const length = () => properties.scale.end - properties.scale.start;

    return (
        <div
            data-slot="event-calendar-timed"
            {...style.attrs(
                styles.event,
                places.box(
                    TimeScale.at(properties.scale, properties.segment.start),
                    (properties.segment.end - properties.segment.start) / length(),
                    properties.segment.position / properties.segment.positions,
                    1 / properties.segment.positions,
                ),
            )}
        >
            {/* The event */}
            <Dynamic
                component={render}
                event={properties.event}
                layout="block"
                isStart={properties.segment.isStart}
                isEnd={properties.segment.isEnd}
                {...mergeProperties(
                    { ref: (created: HTMLButtonElement) => (element = created) },
                    move,
                )}
                xstyle={styles.fill}
            />

            {/* The handle that moves its end */}
            <Show when={end}>
                {(draggable) => (
                    <span
                        tabindex={0}
                        data-slot="event-calendar-resize"
                        aria-label={locale.render(t`Change the end of ${properties.event.title}`)}
                        {...mergeProperties(
                            { ref: (created: HTMLSpanElement) => (handle = created) },
                            draggable().attributes([" "]),
                        )}
                        {...style.attrs(styles.handle)}
                    />
                )}
            </Show>
        </div>
    );
}

/** Render an all-day event in the row of days, moved a day at a time. */
function AllDayEvent(properties: {
    /** The event's segment in the row. */
    readonly segment: Segment<PlainDate>;
    /** The shown days of the row. */
    readonly days: readonly PlainDate[];
}): JSX.Element {
    // drag the event from this part's first day to another of the row
    const control = useEventCalendar();
    const render = useContext(EventCalendarEventContext) ?? EventCalendarEvent;
    const event = () => control.eventOf(properties.segment.key);
    let element: HTMLElement | undefined;
    const draggable = new Draggable(
        {
            id: JSON.stringify([properties.segment.key, properties.segment.start]),
            item: properties.segment.key,
            type: EVENT_TYPE,
            kind: "move",
            label: () => event().title,
            element: () => element,
            home: () => ({ kind: "cell", droppable: DAYS, key: properties.segment.start }),
        },
        control.manager,
    ).attributes([" "]);
    const columns = () => columnsOf(properties.segment, properties.days);

    return (
        <Dynamic
            component={render}
            event={event()}
            layout="inline"
            isStart={properties.segment.isStart}
            isEnd={properties.segment.isEnd}
            {...mergeProperties(
                { ref: (created: HTMLButtonElement) => (element = created) },
                draggable,
            )}
            xstyle={places.span(
                columns().start + 2,
                columns().span,
                properties.segment.position + 1,
            )}
        />
    );
}

/** Step a day target a day along the row of shown days. */
function stepDay(
    target: DragTarget,
    step: DragStep,
    days: readonly PlainDate[],
): DragTarget | undefined {
    // move to the next shown day along the inline axis
    const date = target.kind === "cell" ? target.key : undefined;
    const offset = step === "inline-end" ? 1 : step === "inline-start" ? -1 : 0;
    const index = date === undefined ? -1 : days.indexOf(date);
    const next = offset === 0 || index === -1 ? undefined : days[index + offset];

    return next === undefined ? undefined : { kind: "cell", droppable: DAYS, key: next };
}

/** Round an instant to the nearest step from a day's start. */
function snap(at: number, start: number, step: number): number {
    return start + Math.round((at - start) / step) * step;
}
