import { Icon } from "@destack/icon";
import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, size, space, weight } from "@destack/theme/tokens.stylex";
import { useLocale } from "@destack/locale/solid";
import { text } from "@destack/theme/text";
import type { JSX } from "@solidjs/web";
import { createEffect, createMemo, createSignal, createUniqueId, For, omit } from "solid-js";
import { buttonStyle } from "../button/index.ts";
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
        display: "inline-flex",
        flexDirection: "column",
        gap: space[3],
        padding: space[3],
    },
    header: {
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        gap: space[2],
    },
    caption: {
        margin: 0,
        fontWeight: weight.medium,
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
    "style",
] as const;

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
export interface CalendarLook extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style"> {
    /** The month shown at first, the selection's or today's by default. */
    readonly defaultMonth?: PlainDate;
    /** Report whether a day cannot be selected. */
    readonly isDisabled?: (day: PlainDate) => boolean;
    /** The weekday weeks start on, 1 for Monday to 7 for Sunday, the locale's by default. */
    readonly weekStartsOn?: number;
    /** The day the calendar counts as today, the local clock's by default. */
    readonly today?: PlainDate;
    /** The StyleX styles applied after the calendar's styles. */
    readonly style?: style.Styles;
}

/** The properties of a calendar: its look and the selection of its mode. */
export type CalendarProperties = CalendarLook & CalendarSelection;

/** Render a month grid of days, which arrow keys, Page Up, Page Down, Home and End move through. */
export function Calendar(properties: CalendarProperties): JSX.Element {
    // read the locale's direction and week, and the selected and focused day
    const locale = useLocale();
    const captionId = createUniqueId();
    const today = (): PlainDate => properties.today ?? Day.today();
    const firstDay = (): number => properties.weekStartsOn ?? Day.firstDayOf(locale.tag);
    const [ownValue, setValue] = createSignal<CalendarValue>(properties.defaultValue);
    const value = (): CalendarValue => ("value" in properties ? properties.value : ownValue());
    const [focused, setFocused] = createSignal<PlainDate>(
        properties.defaultMonth ?? firstOf(value()) ?? today(),
        SAME_DAY,
    );
    const month = createMemo(() => Day.monthOf(focused()), SAME_DAY);

    // select a day in the calendar's mode and tell the change handler
    const select = (day: PlainDate) => {
        // compute the selection after the click and keep, focus and report it
        const next = nextValue(properties, value(), day);
        setValue(next);
        setFocused(day);
        report(properties, next);
    };

    return (
        <div
            data-slot="calendar"
            {...omit(properties, ...CALENDAR_OPTIONS)}
            {...style.attrs(text.footnote, styles.calendar, properties.style)}
        >
            <CalendarHeader
                captionId={captionId}
                month={month()}
                direction={locale.direction}
                onMove={(months) => setFocused(Day.addMonths(focused(), months))}
            />
            <CalendarGrid
                captionId={captionId}
                weeks={Day.weeks(month(), firstDay())}
                month={month()}
                focused={focused()}
                stateOf={(day) => stateOf(properties, value(), day)}
                today={today()}
                firstDay={firstDay()}
                isDisabled={(day) => properties.isDisabled?.(day) === true}
                onSelect={select}
                onMove={setFocused}
            />
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
}

/** Render the grid of the month shown, its weekday names over one row per week, which keys move through. */
function CalendarGrid(properties: CalendarGridProperties): JSX.Element {
    // focus the day a key moved to once the grid shows it
    const locale = useLocale();
    let grid: HTMLTableElement | undefined;
    let isMoving = false;
    createEffect(
        () => properties.focused,
        (day) => {
            if (isMoving) {
                grid?.querySelector<HTMLElement>(`[data-day="${Day.key(day)}"]`)?.focus();
                isMoving = false;
            }
        },
    );

    return (
        <table
            role="grid"
            aria-labelledby={properties.captionId}
            onKeyDown={(event) => {
                // move the focused day with the arrow keys, Home, End, Page Up and Page Down
                const next = moveOf(event, properties, locale.direction);
                if (next !== undefined) {
                    event.preventDefault();
                    grid = event.currentTarget;
                    isMoving = true;
                    properties.onMove(next);
                }
            }}
            {...style.attrs(styles.grid)}
        >
            <CalendarWeekdays week={properties.weeks[0] ?? []} />
            <tbody>
                <For each={properties.weeks}>
                    {(week) => (
                        <tr>
                            <For each={week}>
                                {(day) => (
                                    <CalendarCell
                                        day={day}
                                        month={properties.month}
                                        isFocused={PlainDate.equals(day, properties.focused)}
                                        state={properties.stateOf(day)}
                                        isToday={PlainDate.equals(day, properties.today)}
                                        isDisabled={properties.isDisabled(day)}
                                        onSelect={properties.onSelect}
                                    />
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
function CalendarWeekdays(properties: { readonly week: readonly PlainDate[] }): JSX.Element {
    const locale = useLocale();

    return (
        <thead>
            <tr>
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

/** Render the month's caption between the buttons that move to the previous and next month. */
function CalendarHeader(properties: {
    readonly captionId: string;
    readonly month: PlainDate;
    readonly direction: Direction;
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
                onClick={() => properties.onMove(-1)}
                {...style.attrs(buttonStyle({ variant: "ghost", size: "icon-sm" }))}
            >
                <Icon icon={isRightToLeft() ? caretRight : caretLeft} />
            </button>
            <h2
                id={properties.captionId}
                aria-live="polite"
                {...style.attrs(text.footnote, styles.caption)}
            >
                {locale.date(Day.time(properties.month), {
                    month: "long",
                    year: "numeric",
                    timeZone: "UTC",
                })}
            </h2>
            <button
                type="button"
                aria-label={locale.render(t`Go to the next month`)}
                data-slot="calendar-next"
                onClick={() => properties.onMove(1)}
                {...style.attrs(buttonStyle({ variant: "ghost", size: "icon-sm" }))}
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
