import { state } from "@destack/theme/state";
import { t } from "@destack/locale";
import {
    nth,
    PlainDate,
    PlainTime,
    Segment,
    TimeScale,
    type TimeSpan,
    TimeZone,
} from "@destack/schema";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import {
    color,
    radius,
    shadow,
    size,
    space,
    stroke,
    weight,
    swatch,
} from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createMemo,
    createSignal,
    For,
    type JSX,
    omit,
    Show,
    untrack,
    useContext,
    useEnvironment,
    useLocale,
    createEffect,
} from "@destack/view";
import { createElementSize, createResizeObserver } from "@destack/view/primitives/resize-observer";
import { Length } from "../virtualizer/index.ts";
import { Draggable, type DragStep, type DragTarget, Droppable } from "../drag/index.ts";
import { Sweep } from "../interaction/index.ts";
import { mergeProperties } from "../part/index.ts";
import { EventCalendarBackground, EventCalendarEvent, placement } from "./chip.tsx";
import {
    COLUMNS,
    DAY_MILLISECONDS,
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
import { CalendarEvent, type TimedCalendarEvent } from "./event.ts";

/** The height of an hour in the time grid, about twelve in view. */
const HOUR_HEIGHT = `calc(12 * ${space[1]})`;

/** The property with the width of the hour labels beside the day columns, in root text sizes. */
const GUTTER = "--destack-event-calendar-gutter";

/** The least width of a day's column in root text sizes, three beside the hours on a phone. */
const COLUMN_WIDTH = 5;

/** The least height of an event a finger is sure to hit, as WCAG's target size asks for. */
const TOUCH_HEIGHT = size[1];

/** The share of an hour shown above the opening hour, so its label is whole. */
const OPENING_LEAD = 0.25;

/** The hours a day shows, the height of a day of twenty-four. */
const DAY_HOURS = 24;

/** The least height of a row of all-day events, a line of extra-small text and its padding. */
const ALL_DAY_ROW = size[1];

/** The milliseconds of a minute, which the pointer's time rounds to. */
const MINUTE = 60_000;

/** The minutes of an hour. */
const HOUR_MINUTES = 60;

/** The time around now an hour label gives way to the label of now: a quarter hour. */
const NOW_LABEL_REACH = 15 * MINUTE;

/** The droppable of the all-day row's days. */
const DAYS = "event-calendar-days";

/** The views the part shows. */
const TIME_VIEWS: readonly EventCalendarView[] = ["day", "days", "week"];

/** A calendar as narrow as a phone. */
const NARROW_QUERY = "@container event-calendar (max-width: 36rem)";

/** The styles of a time grid. */
const styles = style.create({
    grid: {
        [GUTTER]: { default: "4", [NARROW_QUERY]: "2.75" },
        display: "flex",
        flexDirection: "column",
        minHeight: 0,
        borderBlockWidth: stroke.border,
        borderInlineWidth: { default: stroke.border, [NARROW_QUERY]: 0 },
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: { default: radius[4], [NARROW_QUERY]: 0 },
        overflow: "hidden",
    },
    row: {
        display: "grid",
        borderBlockEndWidth: stroke.border,
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
    },
    resource: {
        paddingBlock: space[1],
        textAlign: "center",
        fontWeight: weight.medium,
        borderInlineStartWidth: stroke.border,
        borderInlineStartStyle: "solid",
        borderInlineStartColor: color.border,
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
        fontWeight: weight.medium,
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
    zones: {
        display: "flex",
    },
    gutter: {
        display: "flex",
        flexDirection: "column",
        justifyContent: "flex-end",
        gap: space[1],
        paddingBlock: space[1],
        color: color.mutedForeground,
    },
    weekNumber: {
        paddingInline: space[2],
    },
    zoneLabel: {
        flexGrow: 1,
        flexBasis: 0,
        textAlign: "end",
        paddingInlineEnd: space[2],
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    hours: {
        position: "relative",
        flexGrow: 1,
        flexBasis: 0,
    },
    hoverLabel: {
        paddingInline: space[1],
        borderRadius: radius[1],
        backgroundColor: color.foreground,
        color: color.background,
        fontWeight: weight.medium,
    },
    hoverLine: {
        position: "absolute",
        insetInline: 0,
        borderBlockStartWidth: stroke.border,
        borderBlockStartStyle: "solid",
        borderBlockStartColor: color.mutedForeground,
        zIndex: 2,
        pointerEvents: "none",
    },
    hour: {
        position: "absolute",
        insetInlineEnd: { default: space[2], [NARROW_QUERY]: space[1] },
        fontSize: { default: null, [NARROW_QUERY]: `calc(11 / 4 * ${space[1]})` },
        transform: "translateY(-50%)",
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
    },
    column: {
        position: "relative",
        userSelect: "none",
        borderInlineStartWidth: stroke.border,
        borderInlineStartStyle: "solid",
        borderInlineStartColor: color.border,
        cursor: "cell",
    },
    line: {
        position: "absolute",
        insetInline: 0,
        borderBlockStartWidth: stroke.border,
        borderBlockStartStyle: "solid",
        borderBlockStartColor: `color-mix(in oklab, ${color.border} 60%, transparent)`,
    },
    event: {
        containerType: "size",
        position: "absolute",
        minHeight: {
            default: `calc(${space[4]} + ${space[1]})`,
            [media.pointerCoarse]: TOUCH_HEIGHT,
        },
        paddingInlineEnd: space[1],
    },
    fill: {
        width: "100%",
        height: "100%",
    },
    landing: {
        zIndex: 3,
        borderRadius: radius[2],
        boxShadow: shadow.raised,
        pointerEvents: "none",
    },
    handle: {
        position: "absolute",
        insetInline: 0,
        insetBlockEnd: 0,
        insetBlockStart: "auto",
        height: space[2],
        cursor: "ns-resize",
        zIndex: 2,
    },
    handleStart: {
        insetBlockStart: 0,
        insetBlockEnd: "auto",
    },
    now: {
        position: "absolute",
        insetInline: 0,
        borderBlockStartWidth: stroke.ring,
        borderBlockStartStyle: "solid",
        borderBlockStartColor: swatch.destructiveMark,
        zIndex: 3,
        pointerEvents: "none",
        "::before": {
            content: "''",
            position: "absolute",
            insetInlineStart: `calc(${space[1]} / -2 - ${stroke.ring})`,
            insetBlockStart: `calc(${space[2]} / -2 - ${stroke.ring} / 2)`,
            width: space[2],
            height: space[2],
            borderRadius: radius.full,
            backgroundColor: swatch.destructiveMark,
        },
    },
    nowLabel: {
        outlineStyle: { default: null, [media.forcedColors]: "solid" },
        outlineWidth: { default: null, [media.forcedColors]: stroke.border },
        marginInlineEnd: `calc(-1 * ${space[1]})`,
        paddingInline: { default: space[1], [NARROW_QUERY]: stroke.ring },
        fontSize: { default: null, [NARROW_QUERY]: `calc(10 / 4 * ${space[1]})` },
        letterSpacing: { default: null, [NARROW_QUERY]: "-0.02em" },
        borderRadius: radius[1],
        backgroundColor: swatch.destructiveSolid,
        color: swatch.destructiveLabel,
        fontWeight: weight.medium,
        whiteSpace: "nowrap",
    },
    covered: {
        visibility: "hidden",
    },
    shade: {
        position: "absolute",
    },
    free: {
        display: "flex",
        alignItems: "flex-start",
        justifyContent: "space-between",
        gap: space[1],
        borderWidth: stroke.border,
        borderStyle: "dashed",
        borderColor: swatch.accentMark,
        pointerEvents: "auto",
        zIndex: 2,
    },
    remove: {
        paddingInline: space[1],
        borderRadius: radius[1],
        color: color.mutedForeground,
        lineHeight: 1,
    },
    nonWorking: {
        backgroundColor: `color-mix(in oklab, ${color.muted} 50%, transparent)`,
        pointerEvents: "none",
    },
    slotLine: {
        borderBlockStartStyle: "dashed",
        borderBlockStartColor: `color-mix(in oklab, ${color.border} 35%, transparent)`,
    },
    sweep: {
        position: "absolute",
        insetInline: space[1],
        overflow: "hidden",
        paddingBlock: space[1],
        paddingInline: space[2],
        borderRadius: radius[2],
        backgroundColor: `color-mix(in oklab, ${swatch.accentMark} 20%, transparent)`,
        color: color.foreground,
        whiteSpace: "nowrap",
        fontVariantNumeric: "tabular-nums",
        pointerEvents: "none",
    },
});

/** The places of the time grid's parts. */
const places = style.create({
    hours: (hourHeight: string) => ({ height: `calc(${hourHeight} * ${String(DAY_HOURS)})` }),
    columns: (days: number, zones: number) => ({
        gridTemplateColumns: `calc(var(${GUTTER}) * ${space[4]} * ${String(zones)}) repeat(${String(days)}, minmax(0, 1fr))`,
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

/** A day's column of hours: its time, the drop it takes, and the event swept out over it. */
export class EventCalendarColumnControl {
    /** The calendar. */
    readonly control: EventCalendarControl;
    /** The day. */
    readonly day: PlainDate;
    /** The resource whose column it is, none without resources. */
    readonly resource: string | undefined;
    /** The droppable of the column. */
    readonly id: string;
    /** The span of a new event swept out over empty time, while the pointer sweeps. */
    readonly sweep: Accessor<TimeScale | undefined>;

    /** Make a column of a day, following the span a sweep over it takes. */
    constructor(
        control: EventCalendarControl,
        day: PlainDate,
        resource: string | undefined,
        sweep: Accessor<TimeScale | undefined>,
    ) {
        // take the day and resource, its droppable's id and the sweep over it
        this.control = control;
        this.day = day;
        this.resource = resource;
        this.id = control.column(day, resource);
        this.sweep = sweep;
    }

    /** Read the day's span of time, 23 or 25 hours on a daylight-saving day. */
    scale(): TimeScale {
        return this.control.dayScale(this.day);
    }

    /** Read a span's place on the day's wall clock: its top and its height. */
    box(span: TimeScale): { readonly top: number; readonly height: number } {
        // cut the span to the day
        const wall = this.control.wallSpan(this.day, span);

        return this.control.wallBox({
            start: Math.max(wall.start, 0),
            end: Math.min(wall.end, DAY_MILLISECONDS),
        });
    }

    /** Read the event a drag in progress would leave in this column, at its new time. */
    landing(): TimedCalendarEvent | undefined {
        // leave drags that aim elsewhere alone
        const operation = this.control.manager.operation();
        const target = operation?.target;
        if (
            operation === undefined ||
            operation.type !== EVENT_TYPE ||
            target?.kind !== "time" ||
            target.droppable !== this.id
        ) {
            return undefined;
        }
        const outcome = this.control.outcome({ ...operation, target });
        if (!CalendarEvent.isTimed(outcome)) {
            throw new TypeError(`calendar time drop left '${outcome.key}' without a time`);
        }

        return outcome;
    }
}

/** The column part the column's parts render in. */
const EventCalendarColumnContext = createContext<EventCalendarColumnControl | null>(null);

/** Read the column a part of a day's column renders in. */
export function useEventCalendarColumn(): EventCalendarColumnControl {
    const column = useContext(EventCalendarColumnContext);
    if (column === null) {
        throw new TypeError("a calendar column part needs a calendar column around it");
    }

    return column;
}

/** Render the days of a week or a single day in hours: headings, all-day events and the hours. */
export function EventCalendarTime(properties: EventCalendarPartProperties): JSX.Element {
    return <OfferedView views={TIME_VIEWS}>{() => <TimeView {...properties} />}</OfferedView>;
}

/** Render the time view while it shows. */
function TimeView(properties: EventCalendarPartProperties): JSX.Element {
    // show the days there is room for
    const control = useEventCalendar();
    const rest = omit(properties, "children", "xstyle", "style");
    const locale = useLocale();
    const paging = new Paging(control, () => locale.direction === "rtl");
    const [grid, setGrid] = createSignal<HTMLElement>();
    const gridSize = createElementSize(grid);
    const unit = new Length(() => space[4]);
    createEffect(
        () => [grid(), gridSize.width, unit.pixels(), control.zones().length] as const,
        ([element, gridWidth, pixels, zones]) => {
            // measure in the theme's space, so larger text shows fewer days
            if (element === undefined || gridWidth === null || pixels === undefined) {
                return;
            }
            const hours = Number.parseFloat(getComputedStyle(element).getPropertyValue(GUTTER));
            const columns = Math.floor((gridWidth / pixels - hours * zones) / COLUMN_WIDTH);
            if (gridWidth > 0 && Number.isFinite(columns)) {
                control.setRoom(columns);
            }
        },
    );

    return (
        <div
            data-slot="event-calendar-time"
            {...mergeProperties(mergeProperties({ ref: setGrid }, paging.attributes()), rest)}
            {...style.attributes(
                [styles.grid, paging.style(), properties.xstyle],
                properties.style,
            )}
        >
            {properties.children ?? (
                <>
                    <EventCalendarResourceHeader />
                    <EventCalendarDayHeader />
                    <EventCalendarAllDay />
                    <EventCalendarTimeGrid />
                </>
            )}
            {unit.render()}
        </div>
    );
}

/** Render the row of resource labels above their days' headings, none without resources. */
export function EventCalendarResourceHeader(properties: EventCalendarPartProperties): JSX.Element {
    // label each resource across its shown days
    const control = useEventCalendar();
    const rest = omit(properties, "children", "xstyle", "style");

    return (
        <Show when={(control.options.resources ?? []).length > 0}>
            <div
                data-slot="event-calendar-resource-header"
                {...rest}
                {...style.attributes(
                    [
                        styles.row,
                        places.columns(control.columns().length, control.zones().length),
                        properties.xstyle,
                    ],
                    properties.style,
                )}
            >
                <For each={control.options.resources ?? []}>
                    {(resource, index) => (
                        <span
                            data-slot="event-calendar-resource"
                            data-resource={resource.key}
                            {...style.attrs(
                                text.sm,
                                styles.resource,
                                places.span(
                                    index() * control.days().length + 2,
                                    control.days().length,
                                    1,
                                ),
                            )}
                        >
                            {resource.label}
                        </span>
                    )}
                </For>
            </div>
        </Show>
    );
}

/** Render the row of day headings, each from the function given or as a weekday and a date. */
export function EventCalendarDayHeader(
    properties: EventCalendarDaysProperties & Omit<EventCalendarPartProperties, "children">,
): JSX.Element {
    // head each shown day, the week of the year above the hour labels
    const control = useEventCalendar();
    const locale = useLocale();
    const rest = omit(properties, "children", "xstyle", "style");

    return (
        <div
            data-slot="event-calendar-day-header"
            {...rest}
            {...style.attributes(
                [
                    styles.row,
                    places.columns(control.columns().length, control.zones().length),
                    properties.xstyle,
                ],
                properties.style,
            )}
        >
            {/* The week of the year and the zones' labels above the hour labels */}
            <div {...style.attrs(text.xs, styles.gutter, places.column(1))}>
                <Show when={control.options.weekNumbers === true}>
                    <span
                        data-slot="event-calendar-week-number"
                        {...style.attrs(styles.weekNumber)}
                    >
                        {locale.render(control.weekNumber(nth(control.days(), 0)))}
                    </span>
                </Show>
                <Show when={control.zones().length > 1}>
                    <div {...style.attrs(styles.zones)}>
                        <For each={control.zones()}>
                            {(zone) => (
                                <span
                                    data-slot="event-calendar-zone"
                                    data-zone={zone.zone}
                                    {...style.attrs(styles.zoneLabel)}
                                >
                                    {zone.label ?? zoneName(zone.zone)}
                                </span>
                            )}
                        </For>
                    </div>
                </Show>
            </div>
            <For each={control.columns()}>
                {(column, index) => {
                    const day = {
                        day: column.day,
                        index,
                        ...(column.resource === undefined ? {} : { resource: column.resource }),
                    };

                    return (
                        <EventCalendarDayContext value={day}>
                            {properties.children === undefined ? (
                                <EventCalendarDayHeading />
                            ) : (
                                properties.children(day)
                            )}
                        </EventCalendarDayContext>
                    );
                }}
            </For>
        </div>
    );
}

/** Render a day's heading: its weekday and date, today's marked, opening the day on a press. */
export function EventCalendarDayHeading(
    properties: Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "class"> & {
        /** The StyleX styles applied after the heading's. */
        readonly xstyle?: style.Styles;
    },
): JSX.Element {
    // read the heading's day, marking today
    const control = useEventCalendar();
    const locale = useLocale();
    const { day, index } = useEventCalendarDay();
    const rest = omit(properties, "children", "xstyle", "style");
    const isToday = () => PlainDate.equals(day, control.today());

    return (
        <button
            type="button"
            data-slot="event-calendar-heading"
            data-today={isToday() ? "" : undefined}
            {...mergeProperties({ onClick: () => control.open(day) }, rest)}
            {...style.attributes(
                [
                    text.xs,
                    state.hover,
                    styles.heading,
                    places.column(index() + 2),
                    properties.xstyle,
                ],
                properties.style,
            )}
        >
            {properties.children ?? (
                <>
                    {locale.plainDate(day, { weekday: "short" })}
                    <span {...style.attrs(text.lg, styles.date, isToday() && state.checked)}>
                        {locale.number(PlainDate.day(day))}
                    </span>
                </>
            )}
        </button>
    );
}

/** Render the row of all-day events, moved a day at a time, with whole-day shading behind them. */
export function EventCalendarAllDay(properties: EventCalendarPartProperties): JSX.Element {
    // read the shown days and their whole-day events, stacked in rows
    const control = useEventCalendar();
    const locale = useLocale();
    const rest = omit(properties, "children", "xstyle", "style");
    const days = createMemo(() => control.days());

    // stack each resource's whole-day events over its days, or every event without resources
    const groups = () => {
        const resources = control.options.resources ?? [];

        return resources.length === 0 ? [undefined] : resources.map((resource) => resource.key);
    };
    const stackOf = (background: boolean) =>
        groups().flatMap((resource, group) =>
            control
                .stack(
                    control
                        .events()
                        .filter(
                            (event) =>
                                CalendarEvent.isAllDay(event) &&
                                CalendarEvent.isBackground(event) === background &&
                                control.resourceOf(event)?.key === resource,
                        )
                        .map((event) => CalendarEvent.span(event, control.timeZone())),
                    days(),
                )
                .map((segment) => ({ segment, resource, offset: group * days().length })),
        );
    const stacked = createMemo(() => stackOf(false));
    const shades = createMemo(() => stackOf(true));
    const levels = () => Math.max(1, ...stacked().map((each) => each.segment.positions));

    // take all-day events on the row of days, a day at a time
    let row: HTMLElement | undefined;
    let dayColumns: DOMRect[] = [];
    void new Droppable(
        {
            id: control.dragId(DAYS),
            group: control.dragId(DAYS),
            axis: "inline",
            element: () => row,
            accepts: (draggable) => control.accepts(draggable, "cell"),
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
                // take the day and resource whose column is under the pointer
                const index = dayColumns.findIndex(
                    (bounds) => x >= bounds.left && x < bounds.right,
                );
                const column = control.columns()[index];

                return column === undefined
                    ? undefined
                    : {
                          kind: "cell",
                          droppable: control.dragId(DAYS),
                          key: column.day,
                          ...(column.resource === undefined ? {} : { resource: column.resource }),
                      };
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
            drop: (operation) => control.drop(operation),
        },
        control.manager,
    );

    return (
        <div
            data-slot="event-calendar-all-day"
            {...mergeProperties({ ref: (created: HTMLDivElement) => (row = created) }, rest)}
            {...style.attributes(
                [
                    styles.row,
                    styles.allDay,
                    places.columns(control.columns().length, control.zones().length),
                    properties.xstyle,
                ],
                properties.style,
            )}
        >
            {/* Whole days shaded behind the events */}
            <For each={shades()}>
                {(shade) => {
                    const columns = () => Segment.columns(shade.segment, days());

                    return (
                        <EventCalendarBackground
                            event={control.eventOf(shade.segment.key)}
                            xstyle={places.shade(
                                shade.offset + columns().start + 2,
                                columns().span,
                                levels(),
                            )}
                        />
                    );
                }}
            </For>

            {/* Events */}
            <For
                each={stacked()}
                keyed={(each) => `${control.segmentKey(each.segment)} ${String(each.resource)}`}
            >
                {(each) => (
                    <AllDayEvent
                        segment={each().segment}
                        days={days()}
                        offset={each().offset}
                        resource={each().resource}
                    />
                )}
            </For>
        </div>
    );
}

/** Render the scrolling time grid: the time axis and the day columns by default. */
export function EventCalendarTimeGrid(properties: EventCalendarPartProperties): JSX.Element {
    // read the hour height and the shown days the grid lays out
    const control = useEventCalendar();
    const rest = omit(properties, "children", "xstyle", "style");

    // open on the working day's start once the hours have a height
    const [scroller, setScroller] = createSignal<HTMLElement>();
    let isOpened = false;
    createResizeObserver(scroller, (_rectangle, element) => {
        if (isOpened || element.scrollHeight === 0) {
            return;
        }
        isOpened = true;
        const minutes = PlainTime.minutes(control.scrollTime()) - OPENING_LEAD * HOUR_MINUTES;
        element.scrollTop = (element.scrollHeight * minutes) / (DAY_HOURS * HOUR_MINUTES);
    });

    return (
        <div
            data-slot="event-calendar-time-grid"
            {...mergeProperties({ ref: setScroller }, rest)}
            {...style.attributes([styles.scroller, properties.xstyle], properties.style)}
        >
            <div
                {...style.attrs(
                    styles.body,
                    places.hours(control.options.hourHeight ?? HOUR_HEIGHT),
                    places.columns(control.columns().length, control.zones().length),
                )}
            >
                {properties.children ?? (
                    <>
                        <EventCalendarTimeAxis />
                        <EventCalendarDayColumnList />
                    </>
                )}
            </div>
        </div>
    );
}

/** Render the first shown day's hours in each zone, now and the pointer's time marked. */
export function EventCalendarTimeAxis(properties: EventCalendarPartProperties): JSX.Element {
    // mark each local hour of the day, a daylight-saving day showing 23 or 25
    const control = useEventCalendar();
    const locale = useLocale();
    const rest = omit(properties, "children", "xstyle", "style");
    const scale = () => control.dayScale(nth(control.days(), 0));
    const ticks = () =>
        control
            .hours(scale())
            .filter(
                (tick, index, all) =>
                    index === 0 ||
                    TimeZone.time(control.timeZone(), tick.at) !==
                        TimeZone.time(control.timeZone(), nth(all, index - 1).at),
            );

    // write a time in a zone, with its minutes where the zone runs off the hour
    const format = (at: number, zone: string, isExact: boolean) =>
        locale.date(at, {
            hour: "numeric",
            ...(isExact || !TimeZone.time(zone, at).endsWith(":00") ? { minute: "2-digit" } : {}),
            timeZone: zone,
        });

    // write the time now while today is shown, hiding the hour it covers
    const now = () =>
        control.days().some((day) => PlainDate.equals(day, control.today()))
            ? control.now()
            : undefined;
    const marked = () => control.hovered() ?? now();

    // place an instant at its wall time, whichever day it is on
    const shareOf = (at: number) => control.wallShare(control.dayOf(at), at);
    const shareOfTick = (at: number) => control.wallShare(nth(control.days(), 0), at);
    const isCovered = (at: number): boolean => {
        // compare how far down both are
        const current = marked();
        const reach = shareOfTick(at) - shareOfTick(at - NOW_LABEL_REACH);

        return current !== undefined && Math.abs(shareOf(current) - shareOfTick(at)) < reach;
    };

    return (
        <div
            aria-hidden="true"
            data-slot="event-calendar-time-axis"
            {...rest}
            {...style.attributes([styles.zones, properties.xstyle], properties.style)}
        >
            <For each={control.zones()}>
                {(zone) => (
                    <div
                        data-slot="event-calendar-zone-hours"
                        data-zone={zone.zone}
                        {...style.attrs(styles.hours)}
                    >
                        <For each={ticks()}>
                            {(tick) => (
                                <span
                                    {...style.attrs(
                                        text.xs,
                                        styles.hour,
                                        isCovered(tick.at) && styles.covered,
                                        places.at(shareOfTick(tick.at)),
                                    )}
                                >
                                    {format(tick.at, zone.zone, false)}
                                </span>
                            )}
                        </For>
                        <Show when={marked()}>
                            {(current) => (
                                <span
                                    data-slot={
                                        control.hovered() === undefined
                                            ? "event-calendar-now-label"
                                            : "event-calendar-hover-label"
                                    }
                                    {...style.attrs(
                                        text.xs,
                                        styles.hour,
                                        control.hovered() === undefined
                                            ? styles.nowLabel
                                            : styles.hoverLabel,
                                        places.at(shareOf(current())),
                                    )}
                                >
                                    {format(current(), zone.zone, true)}
                                </span>
                            )}
                        </Show>
                    </div>
                )}
            </For>
        </div>
    );
}

/** Render a line across a column at the wall time under the pointer, in every zone. */
export function EventCalendarHoverLine(properties: EventCalendarPartProperties): JSX.Element {
    // take the pointer's wall time on this column's day
    const control = useEventCalendar();
    const column = useEventCalendarColumn();
    const rest = omit(properties, "children", "xstyle", "style");
    const at = () => {
        const hovered = control.hovered();
        const zone = control.timeZone();

        return hovered === undefined
            ? undefined
            : TimeZone.instant(zone, column.day, TimeZone.time(zone, hovered));
    };

    return (
        <Show when={at()}>
            {(current) => (
                <div
                    data-slot="event-calendar-hover-line"
                    aria-hidden="true"
                    {...rest}
                    {...style.attributes(
                        [
                            styles.hoverLine,
                            places.at(control.wallShare(column.day, current())),
                            properties.xstyle,
                        ],
                        properties.style,
                    )}
                />
            )}
        </Show>
    );
}

/** Render a column per shown day, each from the function given or as the default column. */
export function EventCalendarDayColumnList(properties: EventCalendarDaysProperties): JSX.Element {
    const control = useEventCalendar();

    return (
        <For each={control.columns()}>
            {(column, index) => {
                const day = {
                    day: column.day,
                    index,
                    ...(column.resource === undefined ? {} : { resource: column.resource }),
                };

                return (
                    <EventCalendarDayContext value={day}>
                        {properties.children === undefined ? (
                            <EventCalendarColumn />
                        ) : (
                            properties.children(day)
                        )}
                    </EventCalendarDayContext>
                );
            }}
        </For>
    );
}

/** Render a day's column: hour lines, events, a sweep, a landing and now by default. */
export function EventCalendarColumn(properties: EventCalendarPartProperties): JSX.Element {
    // make the day's column, taking drops and sweeps over it
    const control = useEventCalendar();
    const locale = useLocale();
    const { day, index, resource } = useEventCalendarDay();

    // read the instant under the pointer, and sweep out a new event over empty time, snapped
    const pointerAt = (y: number, box: DOMRect | undefined): number => {
        // refuse a pointer over a column that has no layout yet
        if (box === undefined) {
            throw new TypeError(`calendar column '${column.id}' read before it laid out`);
        }

        return control.wallInstant(day, (y - box.top) / box.height);
    };
    const atPointer = (y: number, box: DOMRect | undefined): number =>
        snap(pointerAt(y, box), scale().start, control.snap);
    const sweep = new Sweep(
        {
            at: (_x, y) => atPointer(y, element?.getBoundingClientRect()),
            span: (anchor, at) => ({
                start: Math.min(anchor, at),
                end: Math.max(anchor + control.snap, at),
            }),
            canStart: (event) =>
                event.target === element &&
                (control.options.onCreate !== undefined || control.options.mode === "availability"),
            onSweep: (span) => control.sweep(span, resource),
        },
        useEnvironment().clock,
    );
    const column = new EventCalendarColumnControl(control, day, resource, sweep.span);
    const scale = () => column.scale();
    const rest = omit(properties, "children", "xstyle", "style");
    let element: HTMLElement | undefined;
    let bounds: DOMRect | undefined;

    // take timed events at a snapped time, moved by the pointer's travel or a snap per arrow key
    void new Droppable(
        {
            id: column.id,
            group: control.dragId(COLUMNS),
            axis: "block",
            element: () => element,
            accepts: (draggable) => control.accepts(draggable, "time"),
            measure: () => {
                bounds = element?.getBoundingClientRect();
            },
            targetAt: (_x, y) => ({
                // take the snapped time under the pointer, on the bounds the last scroll left
                kind: "time",
                droppable: column.id,
                at: atPointer(y, bounds),
                ...(column.resource === undefined ? {} : { resource: column.resource }),
            }),
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
                    : {
                          kind: "time",
                          droppable: column.id,
                          at: next,
                          ...(column.resource === undefined ? {} : { resource: column.resource }),
                      };
            },
            enter: (from) => {
                // take the same local time on this day
                if (from.kind !== "time") {
                    throw new TypeError(`calendar column entered from a ${from.kind} target`);
                }
                const zone = control.timeZone();

                return {
                    kind: "time",
                    droppable: column.id,
                    at: TimeZone.instant(zone, day, TimeZone.time(zone, from.at)),
                    ...(column.resource === undefined ? {} : { resource: column.resource }),
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
            drop: (operation) => control.drop(operation),
        },
        control.manager,
    );

    return (
        <EventCalendarColumnContext value={column}>
            <div
                data-slot="event-calendar-column"
                data-date={day}
                {...mergeProperties(
                    mergeProperties(
                        {
                            ref: (created: HTMLDivElement) => (element = created),
                            onPointerMove: (event: PointerEvent) =>
                                control.setHovered(
                                    snap(
                                        pointerAt(event.clientY, element?.getBoundingClientRect()),
                                        scale().start,
                                        MINUTE,
                                    ),
                                ),
                            onPointerLeave: () => control.setHovered(undefined),
                        },
                        sweep.attributes(),
                    ),
                    rest,
                )}
                {...style.attributes(
                    [styles.column, places.column(index() + 2), properties.xstyle],
                    properties.style,
                )}
            >
                {properties.children ?? (
                    <>
                        <EventCalendarNonWorking />
                        <EventCalendarHourGrid />
                        <EventCalendarHoverLine />
                        <EventCalendarColumnContent />
                        <EventCalendarAvailability />
                        <EventCalendarSweep />
                        <EventCalendarLanding />
                        <EventCalendarNow />
                    </>
                )}
            </div>
        </EventCalendarColumnContext>
    );
}

/** Render a line at each slot of a column's day, the hours' lines stronger. */
export function EventCalendarHourGrid(): JSX.Element {
    const control = useEventCalendar();
    const column = useEventCalendarColumn();

    return (
        <For each={control.gridLines(column.scale())} keyed={(line) => line.at}>
            {(line) => (
                <div
                    aria-hidden="true"
                    data-slot={
                        line().isHour ? "event-calendar-hour-line" : "event-calendar-slot-line"
                    }
                    {...style.attrs(
                        styles.line,
                        !line().isHour && styles.slotLine,
                        places.at(control.wallShare(column.day, line().at)),
                    )}
                />
            )}
        </For>
    );
}

/** Render the time outside the working hours across a column's day as shading. */
export function EventCalendarNonWorking(properties: EventCalendarPartProperties): JSX.Element {
    // shade each span of the day the working hours leave out
    const control = useEventCalendar();
    const column = useEventCalendarColumn();
    const rest = omit(properties, "children", "xstyle", "style");
    const spans = createMemo(() => control.calendar().nonWorking(column.scale()));

    return (
        <For each={spans()} keyed={(span) => span.start}>
            {(span) => (
                <div
                    data-slot="event-calendar-non-working"
                    aria-hidden="true"
                    {...rest}
                    {...style.attributes(
                        [
                            styles.shade,
                            styles.nonWorking,
                            places.box(column.box(span()).top, column.box(span()).height, 0, 1),
                            properties.xstyle,
                        ],
                        properties.style,
                    )}
                />
            )}
        </For>
    );
}

/** Render a column's timed events packed side by side, and its shading events behind them. */
export function EventCalendarColumnContent(): JSX.Element {
    // read the day's timed events, cut to the day and packed side by side
    const control = useEventCalendar();
    const column = useEventCalendarColumn();
    const scale = () => column.scale();
    const spans = (background: boolean): TimeSpan[] =>
        control
            .timedOn(column.day, column.resource)
            .filter((event) => CalendarEvent.isBackground(event) === background)
            .map((event) => ({ key: event.key, start: event.start, end: event.end }));
    const segments = createMemo(() =>
        Segment.pack(
            spans(false).map((span) => ({ key: span.key, ...control.wallSpan(column.day, span) })),
            0,
            DAY_MILLISECONDS,
        ),
    );
    const shades = createMemo(() =>
        spans(true).filter((span) => span.end > scale().start && span.start < scale().end),
    );

    return (
        <>
            {/* Events shown as shading, cut to the day */}
            <For each={shades()}>
                {(span) => {
                    const box = () =>
                        column.box({
                            start: Math.max(span.start, scale().start),
                            end: Math.min(span.end, scale().end),
                        });

                    return (
                        <EventCalendarBackground
                            event={control.eventOf(span.key)}
                            xstyle={[styles.shade, places.box(box().top, box().height, 0, 1)]}
                        />
                    );
                }}
            </For>

            {/* Events */}
            <For each={segments()} keyed={(segment) => control.segmentKey(segment)}>
                {(segment) => (
                    <TimedEvent
                        event={control.timedOf(segment().key)}
                        segment={segment()}
                        day={column.day}
                        column={column.id}
                        resource={column.resource}
                    />
                )}
            </For>
        </>
    );
}

/** Render the time marked free across a column's day, each span taken back by its button. */
export function EventCalendarAvailability(properties: EventCalendarPartProperties): JSX.Element {
    // cut each free span to the day, writing its times
    const control = useEventCalendar();
    const locale = useLocale();
    const column = useEventCalendarColumn();
    const rest = omit(properties, "children", "xstyle", "style");
    const spans = createMemo(() =>
        control.availability
            .spans()
            .filter(
                (span) =>
                    span.resource === column.resource &&
                    span.end > column.scale().start &&
                    span.start < column.scale().end,
            ),
    );

    return (
        <For each={spans()} keyed={(span) => span.start}>
            {(span) => {
                const cut = () => ({
                    start: Math.max(span().start, column.scale().start),
                    end: Math.min(span().end, column.scale().end),
                });
                const times = () =>
                    locale.dateRange(span().start, span().end, {
                        hour: "numeric",
                        minute: "2-digit",
                        timeZone: control.timeZone(),
                    });

                return (
                    <div
                        data-slot="event-calendar-availability"
                        {...rest}
                        {...style.attributes(
                            [
                                text.xs,
                                styles.sweep,
                                styles.free,
                                places.box(column.box(cut()).top, column.box(cut()).height, 0, 1),
                                properties.xstyle,
                            ],
                            properties.style,
                        )}
                    >
                        {times()}
                        <button
                            type="button"
                            data-slot="event-calendar-availability-remove"
                            aria-label={locale.render(t`Take back ${times()}`)}
                            onClick={() => control.availability.unmark(span())}
                            {...style.attrs(state.hover, styles.remove)}
                        >
                            ×
                        </button>
                    </div>
                );
            }}
        </For>
    );
}

/** Render the new event swept out over a column, with the times it spans. */
export function EventCalendarSweep(properties: EventCalendarPartProperties): JSX.Element {
    // read the column's sweep, written as its local times
    const control = useEventCalendar();
    const locale = useLocale();
    const column = useEventCalendarColumn();
    const rest = omit(properties, "children", "xstyle", "style");

    return (
        <Show when={column.sweep()}>
            {(swept) => (
                <div
                    data-slot="event-calendar-sweep"
                    {...rest}
                    {...style.attributes(
                        [
                            text.xs,
                            styles.sweep,
                            places.box(column.box(swept()).top, column.box(swept()).height, 0, 1),
                            properties.xstyle,
                        ],
                        properties.style,
                    )}
                >
                    {properties.children ??
                        locale.dateRange(swept().start, swept().end, {
                            hour: "numeric",
                            minute: "2-digit",
                            timeZone: control.timeZone(),
                        })}
                </div>
            )}
        </Show>
    );
}

/** Render the dragged event where it would land in a column, at its new time. */
export function EventCalendarLanding(properties: EventCalendarPartProperties): JSX.Element {
    // draw the event with the caller's part, retimed to where it would land
    const column = useEventCalendarColumn();
    const Render = useContext(EventCalendarEventContext) ?? EventCalendarEvent;
    const rest = omit(properties, "children", "xstyle", "style");

    return (
        <Show when={column.landing()}>
            {(landed) => (
                <div
                    data-slot="event-calendar-landing"
                    aria-hidden="true"
                    {...rest}
                    {...style.attributes(
                        [
                            styles.event,
                            styles.landing,
                            places.box(column.box(landed()).top, column.box(landed()).height, 0, 1),
                            properties.xstyle,
                        ],
                        properties.style,
                    )}
                >
                    <Render
                        event={landed()}
                        layout="block"
                        isStart={true}
                        isEnd={true}
                        tabindex={-1}
                        xstyle={styles.fill}
                    />
                </div>
            )}
        </Show>
    );
}

/** Render the time now across today's column, with a dot at its start. */
export function EventCalendarNow(properties: EventCalendarPartProperties): JSX.Element {
    // show the line on today's column alone
    const control = useEventCalendar();
    const column = useEventCalendarColumn();
    const rest = omit(properties, "children", "xstyle", "style");

    return (
        <Show when={PlainDate.equals(column.day, control.today()) ? control.now() : undefined}>
            {(now) => (
                <div
                    data-slot="event-calendar-now"
                    aria-hidden="true"
                    {...rest}
                    {...style.attributes(
                        [
                            styles.now,
                            places.at(control.wallShare(column.day, now())),
                            properties.xstyle,
                        ],
                        properties.style,
                    )}
                />
            )}
        </Show>
    );
}

/** Render a timed event in its day's column, moved from anywhere and resized from either edge. */
function TimedEvent(properties: {
    /** The event. */
    readonly event: TimedCalendarEvent;
    /** Its segment in the column, in wall milliseconds into the day. */
    readonly segment: Segment<number>;
    /** The column's day. */
    readonly day: PlainDate;
    /** The column's droppable. */
    readonly column: string;
    /** The resource whose column it is, none without resources. */
    readonly resource: string | undefined;
}): JSX.Element {
    // drag the event's start, or its end from the handle at its foot
    const control = useEventCalendar();
    const Render = useContext(EventCalendarEventContext) ?? EventCalendarEvent;
    const locale = useLocale();
    const key = untrack(() => properties.event.key);
    let element: HTMLElement | undefined;
    let handle: HTMLElement | undefined;
    let startHandle: HTMLElement | undefined;
    const body = new Draggable(
        {
            id: control.dragId(JSON.stringify([key, untrack(() => properties.segment.start)])),
            item: key,
            type: EVENT_TYPE,
            kind: "move",
            label: () => properties.event.title,
            element: () => element,
            home: () => ({
                kind: "time",
                droppable: properties.column,
                at: Math.max(properties.event.start, control.dayScale(properties.day).start),
                ...(properties.resource === undefined ? {} : { resource: properties.resource }),
            }),
        },
        control.manager,
    );
    const isBusy = untrack(() => control.isBusy(properties.event));
    const move = isBusy ? {} : body.attributes([" "]);

    // move the event's start from the part that shows it, and its end from the part with it
    const edge = (kind: "resize-start" | "resize-end", target: () => HTMLElement | undefined) =>
        new Draggable(
            {
                id: control.dragId(JSON.stringify([key, kind])),
                item: key,
                type: EVENT_TYPE,
                kind,
                label: () => properties.event.title,
                element: target,
                home: () => ({
                    kind: "time",
                    droppable: properties.column,
                    at: kind === "resize-start" ? properties.event.start : properties.event.end,
                    ...(properties.resource === undefined ? {} : { resource: properties.resource }),
                }),
            },
            control.manager,
        );
    const start =
        !isBusy && untrack(() => properties.segment.isStart)
            ? edge("resize-start", () => startHandle)
            : undefined;
    const end =
        !isBusy && untrack(() => properties.segment.isEnd)
            ? edge("resize-end", () => handle)
            : undefined;
    const box = () => control.wallBox(properties.segment);

    // forward the drag and the element to the event the caller renders
    const eventAttributes = mergeProperties(
        { ref: (created: HTMLButtonElement) => (element = created) },
        move,
    );

    return (
        <div
            data-slot="event-calendar-timed"
            data-dragging={control.manager.isDragging(key) ? "" : undefined}
            {...style.attrs(
                styles.event,
                control.manager.isDragging(key) && placement.dragging,
                places.box(
                    box().top,
                    box().height,
                    properties.segment.position / properties.segment.positions,
                    1 / properties.segment.positions,
                ),
            )}
        >
            {/* The event */}
            <Render
                event={properties.event}
                layout="block"
                isStart={properties.segment.isStart}
                isEnd={properties.segment.isEnd}
                {...eventAttributes}
                xstyle={styles.fill}
            />

            {/* The handle that moves its start */}
            <Show when={start}>
                {(draggable) => (
                    <span
                        tabindex={0}
                        data-slot="event-calendar-resize-start"
                        aria-label={locale.render(t`Change the start of ${properties.event.title}`)}
                        {...mergeProperties(
                            { ref: (created: HTMLSpanElement) => (startHandle = created) },
                            draggable().attributes([" "]),
                        )}
                        {...style.attrs(styles.handle, styles.handleStart)}
                    />
                )}
            </Show>

            {/* The handle that moves its end */}
            <Show when={end}>
                {(draggable) => (
                    <span
                        tabindex={0}
                        data-slot="event-calendar-resize-end"
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
    /** The columns before its resource's days. */
    readonly offset: number;
    /** The resource whose days it shows in, none without resources. */
    readonly resource: string | undefined;
}): JSX.Element {
    // drag the event from this part's first day to another of the row
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
                droppable: control.dragId(DAYS),
                key: properties.segment.start,
                ...(properties.resource === undefined ? {} : { resource: properties.resource }),
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
            {...eventAttributes}
            xstyle={[
                places.span(
                    properties.offset + columns().start + 2,
                    columns().span,
                    properties.segment.position + 1,
                ),
                control.manager.isDragging(properties.segment.key) && placement.dragging,
            ]}
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

    return next === undefined || target.kind !== "cell" ? undefined : { ...target, key: next };
}

/** Round an instant to the nearest step from a day's start. */
function snap(at: number, start: number, step: number): number {
    return start + Math.round((at - start) / step) * step;
}

/** Read a zone's city, such as New York for America/New_York, which names it short. */
function zoneName(zone: string): string {
    return (zone.split("/").at(-1) ?? zone).replaceAll("_", " ");
}
