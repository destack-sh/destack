import { type Localization, t } from "@destack/locale";
import { PlainDate } from "@destack/schema";
import * as style from "@destack/style";
import {
    createControllableSignal,
    createEffect,
    createSignal,
    type JSX,
    omit,
    untrack,
    useLocale,
} from "@destack/view";
import { Day } from "../calendar/index.ts";
import {
    isDateSegment,
    DateSegmentGroup,
    type DateSegmentFormat,
    type DateSegmentKind,
    type DateSegmentValues,
} from "../date-segment/index.ts";

/** The earliest and latest year a date field takes, as four digits write them. */
const YEARS = [1, 9999] as const;

/** The months of a year. */
const MONTHS = 12;

/** The months of 30 days, the others but February having 31. */
const SHORT_MONTHS: ReadonlySet<number> = new Set([4, 6, 9, 11]);

/** A date whose parts differ, which lays out a locale's date fields. */
const SAMPLE_DATE = Date.UTC(2026, 10, 22);

/** The properties of a date field, the native element's attributes included. */
export interface DateFieldProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "children" | "onChange"
> {
    /** The date, which makes the field controlled, undefined while none is entered. */
    readonly value?: PlainDate | undefined;
    /** The date the field starts from when uncontrolled. */
    readonly defaultValue?: PlainDate;
    /** Handle the date changing: a complete date that exists, or undefined once it is missing or impossible. */
    readonly onValueChange?: (value: PlainDate | undefined) => void;
    /** Whether the field takes no input. */
    readonly disabled?: boolean;
    /** The StyleX styles applied after the field's styles. */
    readonly xstyle?: style.Styles;
}

/** The year, month and day segments of a date in a locale's order, bounds and numerals. */
class DateFormat implements DateSegmentFormat {
    /** The segments and the literals between them in the locale's order. */
    readonly layout: readonly Intl.DateTimeFormatPart[];
    /** The locale the segments write and name themselves in. */
    readonly #locale: Localization;

    /** Lay out a locale's date segments in the Gregorian calendar. */
    constructor(locale: Localization) {
        const format = new Intl.DateTimeFormat(locale.tag, {
            year: "numeric",
            month: "2-digit",
            day: "2-digit",
            calendar: "gregory",
            timeZone: "UTC",
        });
        this.layout = format
            .formatToParts(SAMPLE_DATE)
            .filter((part) => isDateSegment(part) || part.type === "literal");
        this.#locale = locale;
    }

    /** Read the bounds of a segment, a day within its month once the month is known. */
    limits(kind: DateSegmentKind, values: DateSegmentValues): readonly [number, number] {
        // the four-digit years
        if (kind === "year") {
            return YEARS;
        }
        // the months of a year
        else if (kind === "month") {
            return [1, MONTHS];
        }

        return [1, daysIn(values.year, values.month)];
    }

    /** Write a year as its digits, and a month or day in two. */
    text(kind: DateSegmentKind, value: number): string {
        return this.#locale.number(value, {
            minimumIntegerDigits: kind === "year" ? 1 : 2,
            useGrouping: false,
        });
    }

    /** Write a month by its name and a year or day as its number. */
    valueText(kind: DateSegmentKind, value: number): string {
        return kind === "month"
            ? this.#locale.date(Date.UTC(2000, value - 1, 1), { month: "long", timeZone: "UTC" })
            : this.#locale.number(value, { useGrouping: false });
    }

    /** Label a segment by the part of the date it holds. */
    name(kind: DateSegmentKind): string {
        // the year
        if (kind === "year") {
            return this.#locale.render(t`Year`);
        }
        // the month
        else if (kind === "month") {
            return this.#locale.render(t`Month`);
        }

        return this.#locale.render(t`Day`);
    }

    /** Read today's year, month or day. */
    now(kind: DateSegmentKind): number {
        // read the part of today's date
        const today = Day.today();
        if (kind === "year") {
            return today.year;
        }
        // the month
        else if (kind === "month") {
            return today.month;
        }

        return today.day;
    }
}

/** Render a date as one spinbutton per year, month and day in the locale's order, which digits fill and arrow keys step. */
export function DateField(properties: DateFieldProperties): JSX.Element {
    // follow the controlled date, else the field's own, and the segments typed so far
    const format = new DateFormat(useLocale());
    const rest = omit(properties, "value", "defaultValue", "onValueChange");
    const [value, setValue] = createControllableSignal<PlainDate | undefined>({
        isControlled: () => "value" in properties,
        value: () => properties.value,
        defaultValue: properties.defaultValue,
        onChange: (next) => properties.onValueChange?.(next),
    });
    const [values, setValues] = createSignal(valuesOf(untrack(value)), { ownedWrite: true });

    // show a date set from outside, and clear the segments once a shown date is unset
    createEffect(
        value,
        (next, previous) => {
            const held = dateOf(untrack(values));
            if (next !== undefined && !isSame(next, held)) {
                setValues(valuesOf(next));
            } else if (next === undefined && previous !== undefined && isSame(previous, held)) {
                setValues({});
            }
        },
        { defer: true },
    );

    return (
        <DateSegmentGroup
            data-slot="date-field"
            {...rest}
            format={format}
            values={values()}
            onValuesChange={(next, isTyping) => {
                // keep the segments and report the date once it is complete or gone, typing done
                setValues(next);
                const date = dateOf(next);
                if (!isTyping && !isSame(date, value())) {
                    setValue(date);
                }
            }}
        />
    );
}

/** Read the date of the segments, undefined while one is empty or the date does not exist. */
function dateOf(values: DateSegmentValues): PlainDate | undefined {
    // wait for every segment
    const { year, month, day } = values;
    if (year === undefined || month === undefined || day === undefined) {
        return undefined;
    }

    // refuse a date outside its year's months and its month's days
    const isReal =
        year >= YEARS[0] && month >= 1 && month <= MONTHS && day >= 1 && day <= daysIn(year, month);

    return isReal ? { year, month, day } : undefined;
}

/** Split a date into the values of its segments, none for no date. */
function valuesOf(date: PlainDate | undefined): DateSegmentValues {
    return date === undefined ? {} : { year: date.year, month: date.month, day: date.day };
}

/** Report whether two dates are the same day or both absent. */
function isSame(left: PlainDate | undefined, right: PlainDate | undefined): boolean {
    return left === undefined ? right === undefined : PlainDate.equals(left, right);
}

/** Count the days of a month, the most any such month has while the year or month is unknown. */
function daysIn(year: number | undefined, month: number | undefined): number {
    // a month of 30 days, else February by the Gregorian leap rule, else 31
    if (month !== undefined && SHORT_MONTHS.has(month)) {
        return 30;
    } else if (month === 2) {
        const isLeap =
            year === undefined || (year % 4 === 0 && year % 100 !== 0) || year % 400 === 0;

        return isLeap ? 29 : 28;
    }

    return 31;
}
