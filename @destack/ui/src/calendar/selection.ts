import { PlainDate } from "@destack/schema";

/** A span of days from its first to its last, open while only the first is chosen. */
export interface DateRange {
    /** The first day. */
    readonly from: PlainDate;
    /** The last day, undefined until it is chosen. */
    readonly to?: PlainDate;
}

/** A calendar selecting one day. */
export interface CalendarSingle {
    /** Select one day, the default. */
    readonly mode?: "single";
    /** The selected day, which makes the selection controlled, undefined for none. */
    readonly value?: PlainDate | undefined;
    /** The day selected at first when the selection is uncontrolled. */
    readonly defaultValue?: PlainDate;
    /** Handle a day being selected. */
    readonly onValueChange?: (value: PlainDate) => void;
}

/** A calendar selecting any number of days, each click toggling one. */
export interface CalendarMultiple {
    /** Select any number of days. */
    readonly mode: "multiple";
    /** The selected days, which make the selection controlled. */
    readonly value?: readonly PlainDate[];
    /** The days selected at first when the selection is uncontrolled. */
    readonly defaultValue?: readonly PlainDate[];
    /** Handle the selected days changing. */
    readonly onValueChange?: (value: readonly PlainDate[]) => void;
}

/** A calendar selecting a range, the first click choosing its start and the second its end. */
export interface CalendarRange {
    /** Select a range of days. */
    readonly mode: "range";
    /** The selected range, which makes the selection controlled, undefined for none. */
    readonly value?: DateRange | undefined;
    /** The range selected at first when the selection is uncontrolled. */
    readonly defaultValue?: DateRange;
    /** Handle the range changing. */
    readonly onValueChange?: (value: DateRange) => void;
}

/** The selection of a calendar in any mode. */
export type CalendarSelection = CalendarSingle | CalendarMultiple | CalendarRange;

/** The value of a calendar's selection in any mode. */
export type CalendarValue = PlainDate | readonly PlainDate[] | DateRange | undefined;

/** How a day stands in the selection: out of it, selected, or the start, middle or end of a range. */
export type DayState = "none" | "selected" | "start" | "middle" | "end";

/** Read how a day stands in a selection's value. */
export function stateOf(
    selection: CalendarSelection,
    value: CalendarValue,
    day: PlainDate,
): DayState {
    // one or several selected days
    if (value === undefined) {
        return "none";
    } else if (isDays(value)) {
        return value.some((entry) => PlainDate.equals(entry, day)) ? "selected" : "none";
    } else if (!isRange(value)) {
        return PlainDate.equals(value, day) ? "selected" : "none";
    }

    // the ends and the middle of a range
    const end = value.to ?? value.from;
    if (PlainDate.equals(day, value.from)) {
        return "start";
    } else if (PlainDate.equals(day, end)) {
        return "end";
    } else {
        return selection.mode === "range" &&
            PlainDate.compare(day, value.from) > 0 &&
            PlainDate.compare(day, end) < 0
            ? "middle"
            : "none";
    }
}

/** Compute a selection's value after a click on a day. */
export function nextValue(
    selection: CalendarSelection,
    value: CalendarValue,
    day: PlainDate,
): CalendarValue {
    // toggle a day among several
    if (selection.mode === "multiple") {
        const days = isDays(value) ? value : [];

        return days.some((entry) => PlainDate.equals(entry, day))
            ? days.filter((entry) => !PlainDate.equals(entry, day))
            : [...days, day];
    }
    // close an open range at a later day, else start a new one
    else if (selection.mode === "range") {
        const range = value !== undefined && isRange(value) ? value : undefined;

        return range !== undefined &&
            range.to === undefined &&
            PlainDate.compare(day, range.from) >= 0
            ? { from: range.from, to: day }
            : { from: day };
    }
    // take one day
    else {
        return day;
    }
}

/** Report whether a value is a range. */
export function isRange(value: CalendarValue): value is DateRange {
    return value !== undefined && !isDays(value) && "from" in value;
}

/** Report whether a value is a list of days. */
export function isDays(value: CalendarValue): value is readonly PlainDate[] {
    return Array.isArray(value);
}

/** Tell a selection's change handler its new value, typed for its mode. */
export function report(selection: CalendarSelection, value: CalendarValue): void {
    if (selection.mode === "multiple") {
        selection.onValueChange?.(isDays(value) ? value : []);
    } else if (selection.mode === "range") {
        if (isRange(value)) {
            selection.onValueChange?.(value);
        }
    } else if (value !== undefined && !isDays(value) && !isRange(value)) {
        selection.onValueChange?.(value);
    }
}
