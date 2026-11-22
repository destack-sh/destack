import { state } from "@destack/theme/state";
import { plural, t } from "@destack/locale";
import { nth, PlainDate, Week, WEEK_DAYS, MonthGrid, Segment } from "@destack/schema";
import * as style from "@destack/style";
import { color, radius, size, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    createMemo,
    Dynamic,
    For,
    type JSX,
    Show,
    useLocale,
    useContext,
    onCleanup,
    untrack,
} from "@destack/view";
import { Draggable, Droppable } from "../drag/index.ts";
import { mergeProperties } from "../part/index.ts";
import { EventCalendarBackground, EventCalendarEvent } from "./event.tsx";
import {
    columnsOf,
    EVENT_TYPE,
    isAllDay,
    isBackground,
    useEventCalendar,
    EventCalendarEventContext,
} from "./control.ts";

/** The least height of a week of a month, room for a date and three events. */
const WEEK_HEIGHT = "5.5rem";

/** The droppable of a month's days. */
const MONTH = "event-calendar-month";

/** The styles of a month and its days. */
const styles = style.create({
    month: {
        display: "grid",
        flexGrow: 1,
        minHeight: 0,
        overflowY: "auto",
        gridTemplateRows: "auto",
        gridAutoRows: `minmax(${WEEK_HEIGHT}, 1fr)`,
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[4],
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
        gridRow: "1 / -1",
        borderInlineEndWidth: { default: stroke.border, ":last-child": 0 },
        borderInlineEndStyle: "solid",
        borderInlineEndColor: color.border,
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
        marginInline: space[1],
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

/** Render the month of the calendar's date in whole weeks, events stacked over the days. */
export function EventCalendarMonth(): JSX.Element {
    // lay the month out in six weeks from the locale's first day, anew only when the month changes
    const control = useEventCalendar();
    const locale = useLocale();
    const first = createMemo(() => PlainDate.with(control.date(), { day: 1 }));
    const grid = createMemo(() => MonthGrid.of(first(), control.week, "fixed"));
    const weeks = createMemo(() =>
        grid().weeks.map((week) => week.filter((day) => !control.isHidden(day))),
    );
    const spans = createMemo(() =>
        control
            .events()
            .filter((event) => !isBackground(event))
            .map((event) => control.spanOf(event)),
    );
    const shades = createMemo(() =>
        control
            .events()
            .filter((event) => isAllDay(event) && isBackground(event))
            .map((event) => control.spanOf(event)),
    );
    const cells = new Map<string, HTMLElement>();
    const measured = new Map<string, DOMRect>();
    let element: HTMLElement | undefined;

    // take dropped events on the days, stepping a day along a week and a week across weeks
    void new Droppable(
        {
            id: MONTH,
            group: MONTH,
            axis: "both",
            element: () => element,
            accepts: () => true,
            measure: () => {
                measured.clear();
                for (const [key, cell] of cells) {
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
                    : { kind: "cell", droppable: MONTH, key: found[0] };
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
                          droppable: MONTH,
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
        },
        control.manager,
    );

    return (
        <div
            data-slot="event-calendar-month"
            role="grid"
            ref={(created) => (element = created)}
            onKeyDown={(event) =>
                event.defaultPrevented ? undefined : control.focus.move(event, locale.direction)
            }
            onFocusOut={(event) => control.focus.focusOut(event)}
            {...style.attrs(styles.month)}
        >
            {/* Weekday names */}
            <div
                role="row"
                {...style.attrs(styles.weekdays, places.columns(nth(weeks(), 0).length))}
            >
                <For each={weeks()[0]}>
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

            {/* Weeks */}
            <For each={weeks()}>
                {(days) => {
                    const segments = createMemo(() => control.stack(spans(), days));

                    return (
                        <div
                            role="row"
                            data-slot="event-calendar-week"
                            {...style.attrs(
                                styles.week,
                                places.columns(days.length),
                                places.rows(control.dayMaxEvents()),
                            )}
                        >
                            <For each={days}>
                                {(day, column) => (
                                    <MonthDay
                                        day={day}
                                        column={column() + 1}
                                        days={days}
                                        segments={segments()}
                                        isOutside={MonthGrid.isOutside(grid(), day)}
                                        cells={cells}
                                    />
                                )}
                            </For>
                            <For each={control.stack(shades(), days)}>
                                {(segment) => {
                                    const columns = () => columnsOf(segment, days);

                                    return (
                                        <EventCalendarBackground
                                            aria-hidden="true"
                                            event={control.eventOf(segment.key)}
                                            xstyle={places.shade(
                                                columns().start + 1,
                                                columns().span,
                                            )}
                                        />
                                    );
                                }}
                            </For>
                        </div>
                    );
                }}
            </For>
        </div>
    );
}

/** Render a day of a month: its date, the events that start on it, and a link to the rest. */
function MonthDay(properties: {
    /** The day. */
    readonly day: PlainDate;
    /** The day's column in its week, from 1. */
    readonly column: number;
    /** The shown days of its week. */
    readonly days: readonly PlainDate[];
    /** The week's events, stacked. */
    readonly segments: readonly Segment<PlainDate>[];
    /** Whether the day belongs to a neighbouring month. */
    readonly isOutside: boolean;
    /** The elements of the days, which a drag measures. */
    readonly cells: Map<string, HTMLElement>;
}): JSX.Element {
    // focus the day's date as the month's tab stop
    const control = useEventCalendar();
    const locale = useLocale();
    const day = untrack(() => properties.day);
    const limit = () => control.dayMaxEvents();
    let date: HTMLElement | undefined;
    control.focus.bind(
        () => day,
        () => date,
    );
    onCleanup(() => properties.cells.delete(day));

    // read the events that start here, the ones past the limit, and a drag that aims here
    const starting = () =>
        properties.segments.filter(
            (segment) => segment.start === day && segment.position < limit(),
        );
    const hidden = () => Segment.hidden(properties.segments, day, limit());
    const isTarget = (): boolean => {
        const target = control.manager.targetIn(MONTH);

        return target?.kind === "cell" && target.key === day;
    };
    const open = () => {
        control.setDate(day);
        control.setView("day");
    };

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
                data-target={isTarget() ? "" : undefined}
                ref={(created) => properties.cells.set(day, created)}
                {...style.attrs(
                    state.focusInset,
                    styles.day,
                    places.column(properties.column),
                    Week.isWeekend(control.week, day) && styles.weekend,
                    isTarget() && state.target,
                )}
            />

            {/* Date */}
            <button
                type="button"
                data-slot="event-calendar-date"
                data-today={PlainDate.equals(day, control.today()) ? "" : undefined}
                tabindex={control.focus.isActive(day) ? 0 : -1}
                ref={(created) => (date = created)}
                onFocus={() => control.focus.focusIn(day)}
                aria-label={locale.plainDate(day, { dateStyle: "full" })}
                onClick={open}
                {...style.attrs(
                    text.xs,
                    styles.number,
                    places.column(properties.column),
                    properties.isOutside && styles.outside,
                    PlainDate.equals(day, control.today()) && [state.checked, styles.today],
                )}
            >
                {locale.number(PlainDate.day(day))}
            </button>

            {/* Events that start on the day, up to the limit */}
            <For each={starting()}>
                {(segment) => <MonthEvent segment={segment} days={properties.days} />}
            </For>

            {/* A link to the events past the limit */}
            <Show when={hidden() > 0}>
                <button
                    type="button"
                    data-slot="event-calendar-more"
                    onClick={open}
                    {...style.attrs(
                        text.xs,
                        styles.more,
                        places.span(properties.column, 1, limit() + 2),
                    )}
                >
                    {locale.render(t`${plural(hidden(), { one: "+# more", other: "+# more" })}`)}
                </button>
            </Show>
        </div>
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
            home: () => ({ kind: "cell", droppable: MONTH, key: properties.segment.start }),
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
            data-start={properties.segment.start}
            data-end={properties.segment.end}
            {...mergeProperties(
                { ref: (created: HTMLButtonElement) => (element = created) },
                draggable,
            )}
            xstyle={places.span(
                columns().start + 1,
                columns().span,
                properties.segment.position + 2,
            )}
        />
    );
}
