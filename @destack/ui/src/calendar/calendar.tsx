import { Icon } from "@destack/icon";
import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, size, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { useLocale } from "@destack/locale/solid";
import { text } from "@destack/theme/text";
import {
    createControllableSignal,
    createEffect,
    createSignal,
    createUniqueId,
    For,
    type JSX,
    omit,
    Show,
    untrack,
} from "@destack/view";
import { buttonStyle } from "../button/index.ts";
import { Select, SelectItem } from "../select/index.ts";
import type { Direction } from "@destack/locale";
import { PlainDate } from "@destack/schema";
import { Day } from "./day.ts";
import {
    isDays,
    nextValue,
    report,
    stateOf,
    type CalendarSelection,
    type CalendarValue,
    type DayState,
} from "./selection.ts";

/** The styles of a calendar and its days. */
const styles = style.create({
    calendar: {
        position: "relative",
        display: "inline-flex",
        flexWrap: "wrap",
        gap: space[4],
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
    },
    caption: {
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        gap: space[1],
        height: size[2],
        margin: 0,
        fontWeight: weight.medium,
    },
    week: {
        width: size[3],
        color: color.mutedForeground,
        fontWeight: weight.regular,
        fontVariantNumeric: "tabular-nums",
    },
    grid: {
        borderCollapse: "collapse",
    },
    weekday: {
        width: size[3],
        paddingBottom: space[1],
        color: color.mutedForeground,
        fontWeight: weight.regular,
    },
    cell: {
        padding: 0,
        textAlign: "center",
    },
    day: {
        width: size[3],
        height: size[3],
        padding: 0,
        fontWeight: weight.regular,
        fontVariantNumeric: "tabular-nums",
    },
    outside: {
        color: color.mutedForeground,
    },
    today: {
        backgroundColor: color.accent,
        color: color.accentForeground,
    },
    selected: {
        backgroundColor: color.primary,
        color: color.primaryForeground,
    },
    hidden: {
        position: "absolute",
        width: stroke.border,
        height: stroke.border,
        overflow: "hidden",
        clipPath: "inset(50%)",
        whiteSpace: "nowrap",
    },
    middle: {
        borderRadius: 0,
        backgroundColor: color.accent,
        color: color.accentForeground,
    },
});

/** The properties a calendar reads itself instead of passing them to its element. */
const CALENDAR_OPTIONS = [
    "mode",
    "value",
    "defaultValue",
    "onValueChange",
    "defaultMonth",
    "isDisabled",
    "weekStartsOn",
    "today",
    "months",
    "min",
    "max",
    "captionLayout",
    "weekNumbers",
    "outsideDays",
    "xstyle",
    "style",
] as const;

/** The years a year picker reaches back without an earliest day. */
const YEARS_BACK = 100;

/** The years a year picker reaches ahead without a latest day. */
const YEARS_AHEAD = 10;

/** The comparison that keeps a day signal from notifying when it moves to the same day. */
const SAME_DAY = {
    equals: (left: PlainDate, right: PlainDate): boolean => PlainDate.equals(left, right),
};

/** The keys that move the focused day, and the days or months each moves by. */
const MOVES: Readonly<Record<string, (day: PlainDate, isShift: boolean) => PlainDate>> = {
    ArrowUp: (day) => PlainDate.add(day, -7),
    ArrowDown: (day) => PlainDate.add(day, 7),
    PageUp: (day, isShift) => Day.addMonths(day, isShift ? -12 : -1),
    PageDown: (day, isShift) => Day.addMonths(day, isShift ? 12 : 1),
};

/** The month, limits and look of a calendar in any mode, the native element's attributes included. */
export interface CalendarLook extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "ref"> {
    /** The month shown at first, the selection's or today's by default. */
    readonly defaultMonth?: PlainDate;
    /** Report whether a day cannot be selected. */
    readonly isDisabled?: (day: PlainDate) => boolean;
    /** The weekday weeks start on, 1 for Monday to 7 for Sunday, the locale's by default. */
    readonly weekStartsOn?: number;
    /** The day the calendar counts as today, the local clock's by default. */
    readonly today?: PlainDate;
    /** The months shown side by side, one by default. */
    readonly months?: number;
    /** The earliest day that can be selected or shown. */
    readonly min?: PlainDate;
    /** The latest day that can be selected or shown. */
    readonly max?: PlainDate;
    /** How each month's caption reads: its name, or a month and a year picker, its name by default. */
    readonly captionLayout?: "label" | "dropdown";
    /** Whether a column numbers each week by ISO 8601. */
    readonly weekNumbers?: boolean;
    /** Whether the days of neighbouring months fill the first and last weeks, true by default. */
    readonly outsideDays?: boolean;
    /** The StyleX styles applied after the calendar's styles. */
    readonly xstyle?: style.Styles;
}

/** The properties of a calendar: its look and the selection of its mode. */
export type CalendarProperties = CalendarLook & CalendarSelection;

/** Render a month grid of days, which arrow keys, Page Up, Page Down, Home and End move through. */
export function Calendar(properties: CalendarProperties): JSX.Element {
    // read the locale's direction and week, and the selected and focused day
    const locale = useLocale();
    const today = (): PlainDate => properties.today ?? Day.today();
    const firstDay = (): number => properties.weekStartsOn ?? Day.firstDayOf(locale.tag);
    const [value, setValue] = createControllableSignal<CalendarValue>({
        isControlled: () => "value" in properties,
        value: () => properties.value,
        defaultValue: properties.defaultValue,
        onChange: (next) => report(properties, next),
    });
    const [focused, setFocused] = createSignal<PlainDate>(
        Day.clamp(
            properties.defaultMonth ?? firstOf(value()) ?? today(),
            properties.min,
            properties.max,
        ),
        SAME_DAY,
    );
    const count = (): number => Math.max(properties.months ?? 1, 1);

    // keep the first month shown
    const [first, setFirst] = createSignal<PlainDate>(Day.monthOf(untrack(focused)), SAME_DAY);
    const shown = (): PlainDate[] =>
        Array.from({ length: count() }, (_, index) => Day.addMonths(first(), index));
    const isUnavailable = (day: PlainDate): boolean =>
        Day.isOutside(day, properties.min, properties.max) || properties.isDisabled?.(day) === true;

    // move the focus within the allowed days and bring its month into view
    const focus = (day: PlainDate): void => {
        // focus the day within the allowed days
        const next = Day.clamp(day, properties.min, properties.max);
        const month = Day.monthOf(next);
        setFocused(next);

        // move the months shown when the focus leaves them
        if (PlainDate.compare(month, first()) < 0) {
            setFirst(month);
        } else if (PlainDate.compare(month, Day.addMonths(first(), count() - 1)) > 0) {
            setFirst(Day.addMonths(month, 1 - count()));
        }
    };

    // move the months shown and the focus with them
    const shift = (step: number): void => {
        setFirst(Day.addMonths(first(), step));
        setFocused(Day.clamp(Day.addMonths(focused(), step), properties.min, properties.max));
    };

    // focus the day a key moved to once its month shows it
    let root: HTMLDivElement | undefined;
    let isMoving = false;
    const move = (day: PlainDate): void => {
        isMoving = true;
        focus(day);
    };
    createEffect(focused, (day) => {
        // focus the moved-to day once
        if (isMoving) {
            root?.querySelector<HTMLElement>(`[data-day="${Day.key(day)}"]`)?.focus();
            isMoving = false;
        }
    });

    // select a day in the calendar's mode and tell the change handler
    const select = (day: PlainDate) => {
        // focus the day and keep and report the selection after the click
        focus(day);
        setValue(nextValue(properties, value(), day));
    };

    return (
        <div
            data-slot="calendar"
            {...omit(properties, ...CALENDAR_OPTIONS)}
            ref={(element) => {
                root = element;
            }}
            {...style.attributes(
                [text.footnote, styles.calendar, properties.xstyle],
                properties.style,
            )}
        >
            <CalendarHeader
                direction={locale.direction}
                canMove={(step) => {
                    // allow a move whose months still hold an allowed day
                    const target = Day.addMonths(first(), step);
                    const end = PlainDate.add(Day.addMonths(target, count()), -1);

                    return (
                        (properties.min === undefined ||
                            PlainDate.compare(end, properties.min) >= 0) &&
                        (properties.max === undefined ||
                            PlainDate.compare(target, properties.max) <= 0)
                    );
                }}
                onMove={shift}
            />
            <For each={shown()}>
                {(month) => (
                    <CalendarMonth
                        month={month}
                        layout={properties.captionLayout ?? "label"}
                        min={properties.min}
                        max={properties.max}
                        today={today()}
                        onPick={(picked) =>
                            focus(Day.addMonths(focused(), monthsBetween(month, picked)))
                        }
                    >
                        {(captionId) => (
                            <CalendarGrid
                                captionId={captionId}
                                weeks={Day.weeks(month, firstDay())}
                                month={month}
                                focused={focused()}
                                stateOf={(day) => stateOf(properties, value(), day)}
                                today={today()}
                                firstDay={firstDay()}
                                weekNumbers={properties.weekNumbers === true}
                                outsideDays={properties.outsideDays !== false}
                                isDisabled={isUnavailable}
                                onSelect={select}
                                onMove={move}
                            />
                        )}
                    </CalendarMonth>
                )}
            </For>
        </div>
    );
}

/** Count the months from one month to another. */
function monthsBetween(from: PlainDate, to: PlainDate): number {
    return (to.year - from.year) * 12 + (to.month - from.month);
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
    /** The day it is, around which the year picker spans without limits. */
    readonly today: PlainDate;
    /** Handle the person picking another month or year. */
    readonly onPick: (month: PlainDate) => void;
    /** Render the month's grid, named by the caption's id. */
    readonly children: (captionId: string) => JSX.Element;
}): JSX.Element {
    // name the month and list the months and years its pickers offer
    const locale = useLocale();
    const captionId = createUniqueId();
    const name = (month: PlainDate, options: Intl.DateTimeFormatOptions): string =>
        locale.date(Day.time(month), { ...options, timeZone: "UTC" });
    const years = (): number[] => {
        const from = properties.min?.year ?? properties.today.year - YEARS_BACK;
        const to = properties.max?.year ?? properties.today.year + YEARS_AHEAD;

        return Array.from({ length: to - from + 1 }, (_, index) => from + index);
    };
    const isAllowed = (month: PlainDate): boolean =>
        !Day.isOutside(month, properties.min && Day.monthOf(properties.min), properties.max);

    return (
        <div data-slot="calendar-month" {...style.attrs(styles.month)}>
            <h2
                id={captionId}
                aria-live="polite"
                data-slot="calendar-caption"
                {...style.attrs(text.footnote, styles.caption)}
            >
                <Show
                    when={properties.layout === "dropdown"}
                    fallback={name(properties.month, { month: "long", year: "numeric" })}
                >
                    <span {...style.attrs(styles.hidden)}>
                        {name(properties.month, { month: "long", year: "numeric" })}
                    </span>
                    <Select
                        aria-label={locale.render(t`Month`)}
                        value={String(properties.month.month)}
                        onValueChange={(month) =>
                            month !== undefined &&
                            properties.onPick({ ...properties.month, month: Number(month) })
                        }
                    >
                        <For each={Array.from({ length: 12 }, (_, index) => index + 1)}>
                            {(month) => (
                                <SelectItem
                                    value={String(month)}
                                    disabled={!isAllowed({ ...properties.month, month })}
                                >
                                    {name({ ...properties.month, month }, { month: "short" })}
                                </SelectItem>
                            )}
                        </For>
                    </Select>
                    <Select
                        aria-label={locale.render(t`Year`)}
                        value={String(properties.month.year)}
                        onValueChange={(year) =>
                            year !== undefined &&
                            properties.onPick({ ...properties.month, year: Number(year) })
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

/** The month a calendar's grid shows, the day holding the focus and how each day stands. */
interface CalendarGridProperties {
    /** The id of the caption naming the month. */
    readonly captionId: string;
    /** The day weeks start on, 1 for Monday through 7 for Sunday. */
    readonly firstDay: number;
    /** Handle a key moving the focus to a day. */
    readonly onMove: (day: PlainDate) => void;
    /** The weeks shown, each of seven days. */
    readonly weeks: readonly (readonly PlainDate[])[];
    /** A day of the month shown. */
    readonly month: PlainDate;
    /** The day holding the focus. */
    readonly focused: PlainDate;
    /** Read how a day stands in the selection. */
    readonly stateOf: (day: PlainDate) => DayState;
    /** The day it is. */
    readonly today: PlainDate;
    /** Report whether a day is unavailable. */
    readonly isDisabled: (day: PlainDate) => boolean;
    /** Handle a day being chosen. */
    readonly onSelect: (day: PlainDate) => void;
    /** Whether a column numbers each week. */
    readonly weekNumbers: boolean;
    /** Whether the days of neighbouring months fill the first and last weeks. */
    readonly outsideDays: boolean;
}

/** Render the grid of the month shown, its weekday names over one row per week, which keys move through. */
function CalendarGrid(properties: CalendarGridProperties): JSX.Element {
    const locale = useLocale();

    return (
        <table
            role="grid"
            aria-labelledby={properties.captionId}
            onKeyDown={(event) => {
                // move the focused day with the arrow keys, Home, End, Page Up and Page Down
                const next = moveOf(event, properties, locale.direction);
                if (next !== undefined) {
                    event.preventDefault();
                    properties.onMove(next);
                }
            }}
            {...style.attrs(styles.grid)}
        >
            <CalendarWeekdays
                week={properties.weeks[0] ?? []}
                weekNumbers={properties.weekNumbers}
            />
            <tbody>
                <For each={properties.weeks}>
                    {(week) => (
                        <tr>
                            <Show when={properties.weekNumbers}>
                                <th scope="row" {...style.attrs(text.caption, styles.week)}>
                                    {Day.isoWeek(week[3] ?? week[0] ?? properties.month)}
                                </th>
                            </Show>
                            <For each={week}>
                                {(day) => (
                                    <Show
                                        when={
                                            properties.outsideDays ||
                                            day.month === properties.month.month
                                        }
                                        fallback={<td {...style.attrs(styles.cell)} />}
                                    >
                                        <CalendarCell
                                            day={day}
                                            month={properties.month}
                                            isFocused={PlainDate.equals(day, properties.focused)}
                                            state={properties.stateOf(day)}
                                            isToday={PlainDate.equals(day, properties.today)}
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

/** Render the header row of weekday names, abbreviated with their full names. */
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
                    <th
                        scope="col"
                        aria-label={locale.render(t`Week`)}
                        {...style.attrs(text.caption, styles.weekday)}
                    />
                </Show>
                <For each={properties.week}>
                    {(day) => (
                        <th
                            scope="col"
                            abbr={locale.date(Day.time(day), { weekday: "long", timeZone: "UTC" })}
                            {...style.attrs(text.caption, styles.weekday)}
                        >
                            {locale.date(Day.time(day), { weekday: "short", timeZone: "UTC" })}
                        </th>
                    )}
                </For>
            </tr>
        </thead>
    );
}

/** Render the buttons that move the months shown back and forth, unavailable past the earliest and latest days. */
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
            <button
                type="button"
                aria-label={locale.render(t`Go to the previous month`)}
                data-slot="calendar-previous"
                disabled={!properties.canMove(-1)}
                onClick={() => properties.onMove(-1)}
                {...style.attrs(
                    buttonStyle({ variant: "ghost", size: "icon-sm" }),
                    styles.navigation,
                )}
            >
                <Icon icon={isRightToLeft() ? caretRight : caretLeft} />
            </button>
            <button
                type="button"
                aria-label={locale.render(t`Go to the next month`)}
                data-slot="calendar-next"
                disabled={!properties.canMove(1)}
                onClick={() => properties.onMove(1)}
                {...style.attrs(
                    buttonStyle({ variant: "ghost", size: "icon-sm" }),
                    styles.navigation,
                )}
            >
                <Icon icon={isRightToLeft() ? caretLeft : caretRight} />
            </button>
        </div>
    );
}

/** Render one day of the grid as a button named by its full date. */
function CalendarCell(properties: {
    readonly day: PlainDate;
    readonly month: PlainDate;
    readonly isFocused: boolean;
    readonly state: DayState;
    readonly isToday: boolean;
    readonly isDisabled: boolean;
    readonly onSelect: (day: PlainDate) => void;
}): JSX.Element {
    // read how the day stands in the month and the selection
    const locale = useLocale();
    const isOutside = (): boolean => properties.day.month !== properties.month.month;
    const isSelected = (): boolean => properties.state !== "none";
    const isMiddle = (): boolean => properties.state === "middle";
    const isSolid = (): boolean => isSelected() && !isMiddle();

    return (
        <td aria-selected={isSelected() ? "true" : undefined} {...style.attrs(styles.cell)}>
            <button
                type="button"
                tabindex={properties.isFocused ? 0 : -1}
                aria-label={locale.date(Day.time(properties.day), {
                    dateStyle: "full",
                    timeZone: "UTC",
                })}
                aria-current={properties.isToday ? "date" : undefined}
                disabled={properties.isDisabled}
                data-day={Day.key(properties.day)}
                data-range={
                    isSelected() && properties.state !== "selected" ? properties.state : undefined
                }
                data-outside={isOutside() ? "true" : undefined}
                onClick={() => properties.onSelect(properties.day)}
                {...style.attrs(
                    buttonStyle({ variant: "ghost", size: "icon" }),
                    styles.day,
                    isOutside() && styles.outside,
                    properties.isToday && styles.today,
                    isSolid() && styles.selected,
                    isMiddle() && styles.middle,
                )}
            >
                {locale.date(Day.time(properties.day), {
                    day: "numeric",
                    timeZone: "UTC",
                })}
            </button>
        </td>
    );
}

/** Read the day a key moves the focus to, mirroring left and right in right-to-left text. */
function moveOf(
    event: KeyboardEvent,
    grid: Pick<CalendarGridProperties, "focused" | "firstDay">,
    direction: Direction,
): PlainDate | undefined {
    // step a day along the reading direction
    const { focused, firstDay } = grid;
    const step = direction === "rtl" ? -1 : 1;
    if (event.key === "ArrowRight") {
        return PlainDate.add(focused, step);
    } else if (event.key === "ArrowLeft") {
        return PlainDate.add(focused, -step);
    }
    // the start and end of the week
    else if (event.key === "Home") {
        return Day.startOfWeek(focused, firstDay);
    } else if (event.key === "End") {
        return PlainDate.add(Day.startOfWeek(focused, firstDay), 6);
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
        return "from" in value ? value.from : value;
    }
}
