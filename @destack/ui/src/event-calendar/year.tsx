import { state } from "@destack/theme/state";
import { plural, t } from "@destack/locale";
import { MonthGrid, PlainDate, WEEK_DAYS } from "@destack/schema";
import * as style from "@destack/style";
import { color, radius, size, space, swatch, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { createMemo, For, type JSX, omit, useLocale } from "@destack/view";
import {
    type EventCalendarPartProperties,
    type EventCalendarView,
    useEventCalendar,
} from "./control.ts";
import { CalendarEvent } from "./event.ts";
import { OfferedView } from "./view.tsx";
import { mergeProperties } from "../part/index.ts";

/** The views the part shows. */
const YEAR_VIEWS: readonly EventCalendarView[] = ["year"];

/** The months of a year. */
const MONTHS = 12;

/** The narrowest a month of the year takes, room for seven day numbers. */
const MONTH_WIDTH = "13rem";

/** The events past which a day's mark shows at its strongest. */
const BUSY_DAY = 4;

/** The styles of a year. */
const styles = style.create({
    year: {
        display: "grid",
        gridTemplateColumns: `repeat(auto-fill, minmax(${MONTH_WIDTH}, 1fr))`,
        gap: space[6],
        minHeight: 0,
        overflowY: "auto",
        padding: space[2],
    },
    month: {
        display: "grid",
        gridTemplateColumns: "repeat(7, minmax(0, 1fr))",
        rowGap: space[1],
        alignContent: "start",
    },
    title: {
        gridColumn: "1 / -1",
        margin: 0,
        marginBlockEnd: space[1],
        paddingInline: space[1],
        textAlign: "start",
        fontWeight: weight.semibold,
        color: color.foreground,
    },
    weekday: {
        textAlign: "center",
        color: color.mutedForeground,
    },
    day: {
        position: "relative",
        justifySelf: "center",
        minWidth: size[1],
        height: size[1],
        borderRadius: radius.full,
        fontVariantNumeric: "tabular-nums",
        color: color.foreground,
    },
    today: {
        fontWeight: weight.semibold,
    },
    mark: {
        position: "absolute",
        insetBlockEnd: 0,
        insetInlineStart: "50%",
        width: space[1],
        height: space[1],
        marginInlineStart: `calc(${space[1]} / -2)`,
        borderRadius: radius.full,
        backgroundColor: swatch.accentMark,
    },
});

/** The places of a year's parts. */
const places = style.create({
    column: (column: number) => ({ gridColumn: String(column) }),
    busy: (share: number) => ({ opacity: String(0.35 + 0.65 * share) }),
});

/** Render a year as twelve months, each day marked by its events, a press opening it. */
export function EventCalendarYear(properties: EventCalendarPartProperties): JSX.Element {
    return <OfferedView views={YEAR_VIEWS}>{() => <YearView {...properties} />}</OfferedView>;
}

/** Render the year view while it shows. */
function YearView(properties: EventCalendarPartProperties): JSX.Element {
    // count the events of each day of the shown year
    const control = useEventCalendar();
    const locale = useLocale();
    const rest = omit(properties, "children", "xstyle", "style");
    const counts = createMemo(() => {
        // read the year's range
        const days = new Map<PlainDate, number>();
        const range = control.range();
        for (const event of control.events().filter((each) => !CalendarEvent.isBackground(each))) {
            // count the event on each of its days within the year
            const span = CalendarEvent.span(event, control.timeZone());
            const end = PlainDate.compare(span.end, range.to) < 0 ? span.end : range.to;
            for (
                let day = PlainDate.compare(span.start, range.from) > 0 ? span.start : range.from;
                PlainDate.compare(day, end) <= 0;
                day = PlainDate.add(day, { days: 1 })
            ) {
                days.set(day, (days.get(day) ?? 0) + 1);
            }
        }

        return days;
    });

    // list the year's months, and the weekdays of the locale's week
    const months = () => {
        const first = PlainDate.with(control.date(), { month: 1, day: 1 });

        return Array.from({ length: MONTHS }, (_, index) =>
            MonthGrid.of(PlainDate.add(first, { months: index }), control.week, "fit"),
        );
    };
    const weekdays = () => months()[0]?.weeks[0] ?? [];

    return (
        <div
            data-slot="event-calendar-year"
            {...mergeProperties(
                {
                    onKeyDown: (event: KeyboardEvent) =>
                        event.defaultPrevented
                            ? undefined
                            : control.focus.move(event, locale.direction),
                    onFocusOut: (event: FocusEvent & { readonly currentTarget: Element }) =>
                        control.focus.focusOut(event),
                },
                rest,
            )}
            {...style.attributes([styles.year, properties.xstyle], properties.style)}
        >
            <For each={months()}>
                {(grid) => (
                    <section
                        data-slot="event-calendar-year-month"
                        aria-label={locale.plainDate(grid.month, {
                            month: "long",
                            year: "numeric",
                        })}
                        {...style.attrs(styles.month)}
                    >
                        {/* The month's name, which opens it */}
                        <button
                            type="button"
                            tabindex={-1}
                            onClick={() => {
                                control.setDate(grid.month);
                                control.setView("month");
                            }}
                            {...style.attrs(text.sm, state.hover, styles.title)}
                        >
                            {locale.plainDate(grid.month, { month: "long" })}
                        </button>

                        {/* The weekdays */}
                        <For each={weekdays()}>
                            {(day) => (
                                <span aria-hidden="true" {...style.attrs(text.xs, styles.weekday)}>
                                    {locale.plainDate(day, { weekday: "narrow" })}
                                </span>
                            )}
                        </For>

                        {/* The month's days, the weeks' days of other months left blank */}
                        <For each={grid.weeks.flat()}>
                            {(day, index) =>
                                MonthGrid.isOutside(grid, day) ? (
                                    <span aria-hidden="true" />
                                ) : (
                                    <YearDay
                                        day={day}
                                        count={counts().get(day) ?? 0}
                                        column={(index() % WEEK_DAYS) + 1}
                                    />
                                )
                            }
                        </For>
                    </section>
                )}
            </For>
        </div>
    );
}

/** Render a day of a year's month: its number, marked by its events, opening it on a press. */
function YearDay(properties: {
    /** The day. */
    readonly day: PlainDate;
    /** The events on it. */
    readonly count: number;
    /** Its column in its week, from one. */
    readonly column: number;
}): JSX.Element {
    // take the focus as the calendar's date, marking today
    const control = useEventCalendar();
    const locale = useLocale();
    const isToday = () => PlainDate.equals(properties.day, control.today());
    let element: HTMLElement | undefined;
    control.focus.bind(
        () => properties.day,
        () => element,
    );
    const label = () =>
        locale.render(
            t`${locale.plainDate(properties.day, { dateStyle: "full" })}, ${plural(properties.count, { one: "# event", other: "# events" })}`,
        );

    return (
        <button
            type="button"
            data-slot="event-calendar-year-day"
            data-date={properties.day}
            data-count={properties.count}
            data-today={isToday() ? "" : undefined}
            tabindex={control.focus.isActive(properties.day) ? 0 : -1}
            aria-label={label()}
            ref={(created: HTMLButtonElement) => (element = created)}
            onFocus={() => control.focus.focusIn(properties.day)}
            onClick={() => control.open(properties.day)}
            {...style.attrs(
                text.xs,
                state.hover,
                styles.day,
                places.column(properties.column),
                isToday() && [state.checked, styles.today],
            )}
        >
            {locale.number(PlainDate.day(properties.day))}
            {properties.count > 0 ? (
                <span
                    aria-hidden="true"
                    {...style.attrs(
                        styles.mark,
                        places.busy(Math.min(properties.count, BUSY_DAY) / BUSY_DAY),
                    )}
                />
            ) : null}
        </button>
    );
}
