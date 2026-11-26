import { renderPart } from "../part/index.ts";
import { state } from "@destack/theme/state";
import { Icon } from "@destack/ui/icon";
import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, size, space, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createControllableSignal,
    createEffect,
    createMemo,
    createSignal,
    createUniqueId,
    For,
    type JSX,
    omit,
    onSettled,
    type Setter,
    Show,
    untrack,
    useEnvironment,
    useNow,
    useLocale,
} from "@destack/view";
import { Focus, type KeyboardDelegate } from "../focus/index.ts";
import { Button } from "../button/index.ts";
import { Select, SelectItem } from "../select/index.ts";
import type { Direction } from "@destack/locale";
import { PlainDate, TimeZone, type Weekday, Week, WEEK_DAYS, MonthGrid } from "@destack/schema";
import {
    isDays,
    nextValue,
    report,
    stateOf,
    type CalendarSelection,
    type CalendarValue,
    type DayState,
} from "./selection.ts";
import { VisuallyHidden, visuallyHiddenStyle } from "../visually-hidden/index.ts";

/** The query of a page too narrow for two months beside each other. */
const NARROW_QUERY = "@media (max-width: 47.999rem)";

/** The styles of a calendar and its days. */
const styles = style.create({
    calendar: {
        position: "relative",
        display: "inline-flex",
        // stack the months where they do not fit beside each other
        flexDirection: { default: "row", [NARROW_QUERY]: "column" },
        gap: space[4],
        maxWidth: "100%",
        padding: space[3],
    },
    header: {
        position: "absolute",
        insetBlockStart: space[3],
        insetInline: space[3],
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        pointerEvents: "none",
    },
    navigation: {
        pointerEvents: "auto",
    },
    month: {
        display: "flex",
        flexDirection: "column",
        gap: space[3],
        minWidth: 0,
    },
    caption: {
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        gap: space[1],
        height: size[2],
        fontWeight: weight.medium,
    },
    week: {
        color: color.mutedForeground,
        fontWeight: weight.regular,
        fontVariantNumeric: "tabular-nums",
    },
    grid: {
        // shrink the days where the week does not fit
        tableLayout: "fixed",
        maxWidth: "100%",
        borderCollapse: "collapse",
    },
    columns: (count: string) => ({ width: `calc(${count} * ${size[3]})` }),
    weekday: {
        paddingBottom: space[1],
        color: color.mutedForeground,
        fontWeight: weight.regular,
    },
    cell: {
        textAlign: "center",
    },
    today: {
        backgroundColor: color.hover,
        fontWeight: weight.semibold,
    },
    day: {
        width: "100%",
        height: size[3],
        fontWeight: weight.regular,
        fontVariantNumeric: "tabular-nums",
    },
    outside: {
        color: color.mutedForeground,
    },
    middle: {
        borderRadius: 0,
    },
});

/** The properties a calendar reads itself instead of passing them to its element. */
const CALENDAR_OPTIONS = [
    "mode",
    "selected",
    "defaultSelected",
    "onSelect",
    "defaultMonth",
    "disabled",
    "weekStartsOn",
    "today",
    "numberOfMonths",
    "min",
    "max",
    "captionLayout",
    "showWeekNumber",
    "showOutsideDays",
    "autoFocus",
] as const;

/** The years a year picker reaches back without an earliest day. */
const YEARS_BACK = 100;

/** The years a year picker reaches ahead without a latest day. */
const YEARS_AHEAD = 10;

/** The keys that move the focused day, and the days or months each moves by. */
const MOVES: Readonly<Record<string, (day: PlainDate, isShift: boolean) => PlainDate>> = {
    ArrowUp: (day) => PlainDate.add(day, { weeks: -1 }),
    ArrowDown: (day) => PlainDate.add(day, { weeks: 1 }),
    PageUp: (day, isShift) => PlainDate.add(day, { months: isShift ? -12 : -1 }),
    PageDown: (day, isShift) => PlainDate.add(day, { months: isShift ? 12 : 1 }),
};

/** The properties of a calendar: its look and the selection of its mode. */
export type CalendarProperties = Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "ref" | "onSelect"
> & {
    /** The month shown at first, the selection's or today's by default. */
    readonly defaultMonth?: PlainDate;
    /** Report whether a day cannot be selected. */
    readonly disabled?: (day: PlainDate) => boolean;
    /** The weekday weeks start on, 1 for Monday to 7 for Sunday, the locale's by default. */
    readonly weekStartsOn?: Weekday;
    /** The day the calendar counts as today, the page clock's by default. */
    readonly today?: PlainDate;
    /** The months shown side by side, one by default. */
    readonly numberOfMonths?: number;
    /** The earliest day that can be selected or shown. */
    readonly min?: PlainDate;
    /** The latest day that can be selected or shown. */
    readonly max?: PlainDate;
    /** How each month's caption reads: its name, or a month and a year picker. */
    readonly captionLayout?: "label" | "dropdown";
    /** Whether a column numbers each week by ISO 8601. */
    readonly showWeekNumber?: boolean;
    /** Whether the days of neighbouring months fill the first and last weeks, true by default. */
    readonly showOutsideDays?: boolean;
    /** Whether the focused day takes the focus once the calendar mounts. */
    readonly autoFocus?: boolean;
    /** The StyleX styles applied after the calendar's styles. */
    readonly xstyle?: style.Styles;
} & CalendarSelection;

/** The selection, focused day and months shown of a calendar, which its months and grids share. */
export class CalendarControl {
    /** The calendar's properties, read for its mode, limits and change handler. */
    readonly properties: CalendarProperties;
    /** The selection. */
    readonly value: Accessor<CalendarValue>;
    /** The focused day, which has the tab stop. */
    readonly focus: Focus<PlainDate>;
    /** The first month shown. */
    readonly first: Accessor<PlainDate>;
    /** Replace the selection and tell the change handler. */
    readonly #setValue: (value: CalendarValue) => void;
    /** Replace the first month shown. */
    readonly #setFirst: Setter<PlainDate>;
    /** The selection the person last made, which a selection from outside differs from. */
    #selected: CalendarValue = undefined;

    /** Create the state of a calendar from its properties, showing a first month at first. */
    constructor(properties: CalendarProperties, week: Accessor<Week>, month: PlainDate) {
        // follow the controlled selection or the calendar's own
        const [value, setValue] = createControllableSignal<CalendarValue>({
            isControlled: () => "selected" in properties,
            value: () => properties.selected,
            defaultValue: untrack(() => properties.defaultSelected),
            onChange: (next) => report(properties, next),
        });
        const start = untrack(() => PlainDate.clamp(month, properties.min, properties.max));
        const [first, setFirst] = createSignal<PlainDate>(PlainDate.with(start, { day: 1 }));
        this.properties = properties;
        this.value = value;
        this.first = first;
        this.#setValue = setValue;
        this.#setFirst = setFirst;

        // focus the start day, bringing the month of each focused day into view
        this.focus = new Focus({
            delegate: new CalendarDelegate(properties, week, () => start),
            mode: "roving",
            onChange: (day) => this.#show(day),
        });

        // bring a selection set from outside into view, such as a day typed into a date picker
        createEffect(
            value,
            (next) => {
                const day = firstOf(next);
                if (next !== this.#selected && day !== undefined) {
                    this.moveTo(day);
                }
            },
            { defer: true },
        );
    }

    /** The focused day. */
    focused(): PlainDate {
        return this.focus.active() ?? PlainDate.with(this.first(), { day: 1 });
    }

    /** The number of months shown side by side. */
    count(): number {
        return Math.max(this.properties.numberOfMonths ?? 1, 1);
    }

    /** The months shown, from the first. */
    shown(): PlainDate[] {
        return Array.from({ length: this.count() }, (_, index) =>
            PlainDate.add(this.first(), { months: index }),
        );
    }

    /** Report whether a day cannot be selected: outside the limits or disabled by the owner. */
    isUnavailable(day: PlainDate): boolean {
        return (
            isOutsideRange(day, this.properties.min, this.properties.max) ||
            this.properties.disabled?.(day) === true
        );
    }

    /** Report whether moving the months shown by a step still shows an allowed day. */
    canShift(step: number): boolean {
        // read the first and last day the moved months show
        const { min, max } = this.properties;
        const target = PlainDate.add(this.first(), { months: step });
        const end = PlainDate.add(PlainDate.add(target, { months: this.count() }), { days: -1 });

        return (
            (min === undefined || PlainDate.compare(end, min) >= 0) &&
            (max === undefined || PlainDate.compare(target, max) <= 0)
        );
    }

    /** Focus a day within the allowed days, bringing its month into view. */
    moveTo(day: PlainDate): void {
        this.focus.focus(PlainDate.clamp(day, this.properties.min, this.properties.max));
    }

    /** Move the months shown and the focus with them. */
    shift(step: number): void {
        this.#setFirst(PlainDate.add(this.first(), { months: step }));
        this.moveTo(PlainDate.add(this.focused(), { months: step }));
    }

    /** Select a day in the calendar's mode, focusing it and telling the change handler. */
    select(day: PlainDate): void {
        this.moveTo(day);
        this.#selected = nextValue(this.properties, this.value(), day);
        this.#setValue(this.#selected);
    }

    /** Move the months shown when a focused day leaves them. */
    #show(day: PlainDate): void {
        const month = PlainDate.with(day, { day: 1 });
        if (PlainDate.compare(month, this.first()) < 0) {
            this.#setFirst(month);
        } else if (
            PlainDate.compare(month, PlainDate.add(this.first(), { months: this.count() - 1 })) > 0
        ) {
            this.#setFirst(PlainDate.add(month, { months: 1 - this.count() }));
        }
    }
}

/** The days a calendar's keys move the focus through. */
class CalendarDelegate implements KeyboardDelegate<PlainDate> {
    /** The calendar's properties, read for its limits. */
    readonly #properties: CalendarProperties;
    /** The weekday weeks start on. */
    readonly #week: Accessor<Week>;
    /** The day the focus starts on. */
    readonly #start: Accessor<PlainDate>;

    /** Move through a calendar's days within its limits. */
    constructor(properties: CalendarProperties, week: Accessor<Week>, start: Accessor<PlainDate>) {
        this.#properties = properties;
        this.#week = week;
        this.#start = start;
    }

    /** Read the day a key moves the focus to, within the allowed days. */
    target(event: KeyboardEvent, from: PlainDate, direction: Direction): PlainDate | undefined {
        const next = moveOf(event, from, this.#week(), direction);

        return next === undefined
            ? undefined
            : PlainDate.clamp(next, this.#properties.min, this.#properties.max);
    }

    /** Read the day the focus starts on. */
    first(): PlainDate {
        return this.#start();
    }

    /** Report whether a day lies within the allowed days. */
    has(day: PlainDate): boolean {
        return !isOutsideRange(day, this.#properties.min, this.#properties.max);
    }
}

/** Render a month grid of days, which arrow keys, Page Up, Page Down, Home and End move through. */
export function Calendar(properties: CalendarProperties): JSX.Element {
    // read today once mounted, so the server and the browser render the same
    const environment = useEnvironment();
    const now = useNow();
    const today = createMemo((): PlainDate | undefined => {
        // take the given day, else the clock's once the view knows the time
        if (properties.today !== undefined) {
            return properties.today;
        }
        const time = now();

        return time === undefined ? undefined : TimeZone.date(environment.timeZone, time);
    });

    // take the first start: the given month, the selection's, or today's once known
    const given = untrack(
        () =>
            properties.defaultMonth ??
            firstOf("selected" in properties ? properties.selected : properties.defaultSelected),
    );
    const start = createMemo<PlainDate | undefined>((previous) => previous ?? given ?? today());

    return (
        <Show when={start()} keyed>
            {(month) => <CalendarMonths calendar={properties} month={month} today={today} />}
        </Show>
    );
}

/** Render a started calendar's months from its first month, marking today once known. */
function CalendarMonths(properties: {
    /** The calendar's properties. */
    readonly calendar: CalendarProperties;
    /** The first month shown at first. */
    readonly month: PlainDate;
    /** The day it is, absent before the view knows the time. */
    readonly today: Accessor<PlainDate | undefined>;
}): JSX.Element {
    // read the locale's direction and week, and the calendar's state
    const calendar = untrack(() => properties.calendar);
    const locale = useLocale();
    const week = (): Week =>
        calendar.weekStartsOn === undefined
            ? locale.week
            : { ...locale.week, firstDay: calendar.weekStartsOn };
    const control = new CalendarControl(
        calendar,
        week,
        untrack(() => properties.month),
    );
    const today = untrack(() => properties.today);

    // focus the focused day once mounted when asked
    onSettled(() => {
        if (calendar.autoFocus === true) {
            control.focus.enter(control.focused());
        }
    });

    return renderPart(
        "div",
        "calendar",
        omit(calendar, ...CALENDAR_OPTIONS),
        [text.xs, styles.calendar],
        {
            onFocusOut: (event) => control.focus.focusOut(event),
            get children() {
                return (
                    <>
                        <CalendarHeader
                            direction={locale.direction}
                            canMove={(step) => control.canShift(step)}
                            onMove={(step) => control.shift(step)}
                        />
                        <For each={control.shown()}>
                            {(month) => (
                                <CalendarMonth
                                    month={month}
                                    layout={calendar.captionLayout ?? "label"}
                                    min={calendar.min}
                                    max={calendar.max}
                                    onPick={(picked) =>
                                        control.moveTo(
                                            PlainDate.add(control.focused(), {
                                                months: monthsBetween(month, picked),
                                            }),
                                        )
                                    }
                                >
                                    {(captionId) => (
                                        <CalendarGrid
                                            captionId={captionId}
                                            weeks={MonthGrid.of(month, week(), "fit").weeks}
                                            week={week()}
                                            month={month}
                                            focus={control.focus}
                                            stateOf={(day) =>
                                                stateOf(calendar, control.value(), day)
                                            }
                                            today={today()}
                                            weekNumbers={calendar.showWeekNumber === true}
                                            outsideDays={calendar.showOutsideDays !== false}
                                            isDisabled={(day) => control.isUnavailable(day)}
                                            onSelect={(day) => control.select(day)}
                                        />
                                    )}
                                </CalendarMonth>
                            )}
                        </For>
                    </>
                );
            },
        },
    );
}

/** Count the months from one month to another. */
function monthsBetween(from: PlainDate, to: PlainDate): number {
    return (
        (PlainDate.year(to) - PlainDate.year(from)) * 12 +
        (PlainDate.month(to) - PlainDate.month(from))
    );
}

/** Render one month: its caption, as its name or a month and year picker, over its grid. */
function CalendarMonth(properties: {
    /** The first day of the month. */
    readonly month: PlainDate;
    /** How the caption reads. */
    readonly layout: "label" | "dropdown";
    /** The earliest day allowed. */
    readonly min: PlainDate | undefined;
    /** The latest day allowed. */
    readonly max: PlainDate | undefined;
    /** Handle the person picking another month or year. */
    readonly onPick: (month: PlainDate) => void;
    /** Render the month's grid, named by the caption's id. */
    readonly children: (captionId: string) => JSX.Element;
}): JSX.Element {
    // label the month and list the months and years its pickers offer
    const locale = useLocale();
    const captionId = createUniqueId();
    const name = (month: PlainDate, options: Intl.DateTimeFormatOptions): string =>
        locale.plainDate(month, options);
    const years = (): number[] => {
        // span the years around the shown month
        const center = PlainDate.year(properties.month);
        const from =
            properties.min === undefined ? center - YEARS_BACK : PlainDate.year(properties.min);
        const to =
            properties.max === undefined ? center + YEARS_AHEAD : PlainDate.year(properties.max);

        return Array.from({ length: to - from + 1 }, (_, index) => from + index);
    };
    const isAllowed = (month: PlainDate): boolean =>
        !isOutsideRange(
            month,
            properties.min === undefined ? undefined : PlainDate.with(properties.min, { day: 1 }),
            properties.max,
        );

    return (
        <div data-slot="calendar-month" {...style.attrs(styles.month)}>
            <h2
                id={captionId}
                aria-live="polite"
                data-slot="calendar-caption"
                {...style.attrs(text.sm, styles.caption)}
            >
                <Show
                    when={properties.layout === "dropdown"}
                    fallback={name(properties.month, { month: "long", year: "numeric" })}
                >
                    <span {...style.attrs(visuallyHiddenStyle())}>
                        {name(properties.month, { month: "long", year: "numeric" })}
                    </span>
                    <Select
                        aria-label={locale.render(t.context("label")`Month`)}
                        value={String(PlainDate.month(properties.month))}
                        onValueChange={(month) =>
                            month !== undefined &&
                            properties.onPick(
                                PlainDate.with(properties.month, { month: Number(month) }),
                            )
                        }
                    >
                        <For each={Array.from({ length: 12 }, (_, index) => index + 1)}>
                            {(month) => (
                                <SelectItem
                                    value={String(month)}
                                    disabled={
                                        !isAllowed(PlainDate.with(properties.month, { month }))
                                    }
                                >
                                    {name(PlainDate.with(properties.month, { month }), {
                                        month: "short",
                                    })}
                                </SelectItem>
                            )}
                        </For>
                    </Select>
                    <Select
                        aria-label={locale.render(t.context("label")`Year`)}
                        value={String(PlainDate.year(properties.month))}
                        onValueChange={(year) =>
                            year !== undefined &&
                            properties.onPick(
                                PlainDate.with(properties.month, { year: Number(year) }),
                            )
                        }
                    >
                        <For each={years()}>
                            {(year) => <SelectItem value={String(year)}>{year}</SelectItem>}
                        </For>
                    </Select>
                </Show>
            </h2>
            {properties.children(captionId)}
        </div>
    );
}

/** The month a calendar's grid shows, the focused day and how each day stands. */
interface CalendarGridProperties {
    /** The id of the caption naming the month. */
    readonly captionId: string;
    /** The weeks shown, each of seven days. */
    readonly weeks: readonly (readonly PlainDate[])[];
    /** A day of the month shown. */
    readonly month: PlainDate;
    /** The focused day, which the keys move. */
    readonly focus: Focus<PlainDate>;
    /** Read how a day stands in the selection. */
    readonly stateOf: (day: PlainDate) => DayState;
    /** The day it is, absent before the view knows the time. */
    readonly today: PlainDate | undefined;
    /** Report whether a day is unavailable. */
    readonly isDisabled: (day: PlainDate) => boolean;
    /** Handle a day being chosen. */
    readonly onSelect: (day: PlainDate) => void;
    /** Whether a column numbers each week. */
    readonly weekNumbers: boolean;
    /** Whether the days of neighbouring months fill the first and last weeks. */
    readonly outsideDays: boolean;
    /** The weeks of the locale, which number the rows. */
    readonly week: Week;
}

/** Render the grid of the month shown, which keys move through. */
function CalendarGrid(properties: CalendarGridProperties): JSX.Element {
    const locale = useLocale();

    return (
        <table
            role="grid"
            aria-labelledby={properties.captionId}
            onKeyDown={(event) => properties.focus.move(event, locale.direction)}
            {...style.attrs(styles.grid, styles.columns(properties.weekNumbers ? "8" : "7"))}
        >
            <CalendarWeekdays
                week={properties.weeks[0] ?? []}
                weekNumbers={properties.weekNumbers}
            />
            <tbody>
                <For each={properties.weeks}>
                    {(row) => (
                        <tr>
                            <Show when={properties.weekNumbers}>
                                <th scope="row" {...style.attrs(text.xs, styles.week)}>
                                    {Week.weekOfYear(properties.week, row[0] ?? properties.month)}
                                </th>
                            </Show>
                            <For each={row}>
                                {(day) => (
                                    <Show
                                        when={
                                            properties.outsideDays ||
                                            PlainDate.month(day) ===
                                                PlainDate.month(properties.month)
                                        }
                                        fallback={<td {...style.attrs(styles.cell)} />}
                                    >
                                        <CalendarCell
                                            day={day}
                                            month={properties.month}
                                            focus={properties.focus}
                                            state={properties.stateOf(day)}
                                            isToday={
                                                properties.today !== undefined &&
                                                PlainDate.equals(day, properties.today)
                                            }
                                            isDisabled={properties.isDisabled(day)}
                                            onSelect={properties.onSelect}
                                        />
                                    </Show>
                                )}
                            </For>
                        </tr>
                    )}
                </For>
            </tbody>
        </table>
    );
}

/** Render the header row of weekdays, abbreviated and labelled in full. */
function CalendarWeekdays(properties: {
    /** A week of the month, whose days name the columns. */
    readonly week: readonly PlainDate[];
    /** Whether a column numbers each week. */
    readonly weekNumbers: boolean;
}): JSX.Element {
    const locale = useLocale();

    return (
        <thead>
            <tr>
                <Show when={properties.weekNumbers}>
                    <th scope="col" {...style.attrs(text.xs, styles.weekday)}>
                        <VisuallyHidden>{locale.render(t.context("label")`Week`)}</VisuallyHidden>
                    </th>
                </Show>
                <For each={properties.week}>
                    {(day) => (
                        <th
                            scope="col"
                            abbr={locale.plainDate(day, { weekday: "long" })}
                            {...style.attrs(text.xs, styles.weekday)}
                        >
                            {locale.plainDate(day, { weekday: "short" })}
                        </th>
                    )}
                </For>
            </tr>
        </thead>
    );
}

/** Render the buttons that move the months shown back and forth. */
function CalendarHeader(properties: {
    /** The reading direction, which turns the arrows. */
    readonly direction: Direction;
    /** Report whether the months can move by a number of months. */
    readonly canMove: (months: number) => boolean;
    /** Handle a move by a number of months. */
    readonly onMove: (months: number) => void;
}): JSX.Element {
    const locale = useLocale();
    const isRightToLeft = (): boolean => properties.direction === "rtl";

    return (
        <div data-slot="calendar-header" {...style.attrs(styles.header)}>
            <Button
                variant="ghost"
                isIconOnly
                aria-label={locale.render(t`Go to the previous month`)}
                data-slot="calendar-previous"
                disabled={!properties.canMove(-1)}
                onClick={() => properties.onMove(-1)}
                xstyle={styles.navigation}
            >
                <Icon icon={isRightToLeft() ? caretRight : caretLeft} />
            </Button>
            <Button
                variant="ghost"
                isIconOnly
                aria-label={locale.render(t`Go to the next month`)}
                data-slot="calendar-next"
                disabled={!properties.canMove(1)}
                onClick={() => properties.onMove(1)}
                xstyle={styles.navigation}
            >
                <Icon icon={isRightToLeft() ? caretLeft : caretRight} />
            </Button>
        </div>
    );
}

/** Render one day of the grid as a button named by its full date. */
function CalendarCell(properties: {
    readonly day: PlainDate;
    readonly month: PlainDate;
    readonly focus: Focus<PlainDate>;
    readonly state: DayState;
    readonly isToday: boolean;
    readonly isDisabled: boolean;
    readonly onSelect: (day: PlainDate) => void;
}): JSX.Element {
    // read how the day stands in the month and the selection, taking the focus as the keys reach it
    const locale = useLocale();
    let element: HTMLButtonElement | undefined;
    properties.focus.bind(
        () => properties.day,
        () => element,
    );
    const isOutside = (): boolean =>
        PlainDate.month(properties.day) !== PlainDate.month(properties.month);
    const isSelected = (): boolean => properties.state !== "none";
    const isMiddle = (): boolean => properties.state === "middle";
    const isSolid = (): boolean => isSelected() && !isMiddle();

    return (
        <td aria-selected={isSelected() ? "true" : undefined} {...style.attrs(styles.cell)}>
            <Button
                variant="ghost"
                isIconOnly
                tabindex={properties.focus.isActive(properties.day) ? 0 : -1}
                aria-label={locale.plainDate(properties.day, { dateStyle: "full" })}
                aria-current={properties.isToday ? "date" : undefined}
                disabled={properties.isDisabled}
                data-day={properties.day}
                data-range={
                    isSelected() && properties.state !== "selected" ? properties.state : undefined
                }
                data-outside={isOutside() ? "" : undefined}
                ref={(button) => (element = button)}
                onFocus={() => properties.focus.focusIn(properties.day)}
                onClick={() => properties.onSelect(properties.day)}
                xstyle={[
                    state.unframed,
                    styles.day,
                    isOutside() && styles.outside,
                    properties.isToday && styles.today,
                    isSolid() && state.checked,
                    isMiddle() && [state.selected, styles.middle],
                ]}
            >
                {locale.plainDate(properties.day, { day: "numeric" })}
            </Button>
        </td>
    );
}

/** Report whether a date lies outside the earliest and latest dates allowed. */
function isOutsideRange(
    date: PlainDate,
    earliest: PlainDate | undefined,
    latest: PlainDate | undefined,
): boolean {
    return !PlainDate.equals(PlainDate.clamp(date, earliest, latest), date);
}

/** Read the day a key moves the focus to, mirroring left and right in right-to-left text. */
function moveOf(
    event: KeyboardEvent,
    focused: PlainDate,
    week: Week,
    direction: Direction,
): PlainDate | undefined {
    // step a day along the reading direction
    const step = direction === "rtl" ? -1 : 1;
    if (event.key === "ArrowRight") {
        return PlainDate.add(focused, { days: step });
    } else if (event.key === "ArrowLeft") {
        return PlainDate.add(focused, { days: -step });
    }
    // the start and end of the week
    else if (event.key === "Home") {
        return Week.startOf(week, focused);
    } else if (event.key === "End") {
        return PlainDate.add(Week.startOf(week, focused), { days: WEEK_DAYS - 1 });
    }
    // weeks, months and years
    else {
        return MOVES[event.key]?.(focused, event.shiftKey);
    }
}

/** Read the first selected day of a value, where the calendar opens. */
function firstOf(value: CalendarValue): PlainDate | undefined {
    if (value === undefined) {
        return undefined;
    } else if (isDays(value)) {
        return value[0];
    } else {
        return typeof value === "string" ? value : value.from;
    }
}
