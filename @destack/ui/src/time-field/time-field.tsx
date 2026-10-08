import { type Localization, t } from "@destack/locale";
import { PlainTime } from "@destack/schema";
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
import {
    isDateSegment,
    DateSegmentGroup,
    type DateSegmentFormat,
    type DateSegmentKind,
    type DateSegmentValues,
} from "../date-segment/index.ts";

/** The hours of half a day. */
const HALF_DAY = 12;

/** The minutes of an hour. */
const HOUR_MINUTES = 60;

/** A time in the afternoon whose parts differ, which lays out a locale's time fields. */
const SAMPLE_TIME = Date.UTC(2026, 0, 1, 13, 45);

/** A time in the morning, whose day period is the morning. */
const MORNING = Date.UTC(2026, 0, 1, 1);

/** The properties of a time field, the native element's attributes included. */
export interface TimeFieldProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "children" | "onChange"
> {
    /** The time of day, which makes the field controlled, undefined while none is entered. */
    readonly value?: PlainTime | undefined;
    /** The time the field starts from when uncontrolled. */
    readonly defaultValue?: PlainTime;
    /** Handle the time changing: a complete time, or undefined once a segment is cleared. */
    readonly onValueChange?: (value: PlainTime | undefined) => void;
    /** Whether the field takes no input. */
    readonly disabled?: boolean;
    /** The StyleX styles applied after the field's styles. */
    readonly xstyle?: style.Styles;
}

/** The hour, minute and day period segments of a time in a locale's order, hour cycle and numerals. */
class TimeFormat implements DateSegmentFormat {
    /** The segments and the literals between them in the locale's order. */
    readonly layout: readonly Intl.DateTimeFormatPart[];
    /** Whether hours count 1 to 12 beside a day period, else 0 to 23. */
    readonly isTwelveHour: boolean;
    /** The locale the segments write and name themselves in. */
    readonly #locale: Localization;
    /** The format that writes the day periods. */
    readonly #format: Intl.DateTimeFormat;

    /** Lay out a locale's time segments on its hour cycle. */
    constructor(locale: Localization) {
        // read the locale's hour cycle, a 12-hour clock showing a day period
        const cycle = new Intl.DateTimeFormat(locale.tag, {
            hour: "numeric",
            timeZone: "UTC",
        }).resolvedOptions().hourCycle;
        this.isTwelveHour = cycle === "h11" || cycle === "h12";
        const format = new Intl.DateTimeFormat(locale.tag, {
            hour: "2-digit",
            minute: "2-digit",
            hourCycle: this.isTwelveHour ? "h12" : "h23",
            timeZone: "UTC",
        });

        // keep the segments and the literals
        this.layout = format
            .formatToParts(SAMPLE_TIME)
            .filter((part) => isDateSegment(part) || part.type === "literal");
        this.#locale = locale;
        this.#format = format;
    }

    /** Read the bounds of a segment on the locale's hour cycle. */
    limits(kind: DateSegmentKind): readonly [number, number] {
        // the hours of the clock
        if (kind === "hour") {
            return this.isTwelveHour ? [1, HALF_DAY] : [0, 2 * HALF_DAY - 1];
        }
        // the minutes of an hour
        else if (kind === "minute") {
            return [0, HOUR_MINUTES - 1];
        }

        return [0, 1];
    }

    /** Write an hour or minute in two digits, and a day period as the locale writes it. */
    text(kind: DateSegmentKind, value: number): string {
        // write a number
        if (kind !== "dayPeriod") {
            return this.#locale.number(value, { minimumIntegerDigits: 2, useGrouping: false });
        }

        // label the morning or the afternoon
        const time = value === 0 ? MORNING : SAMPLE_TIME;
        const period = this.#format.formatToParts(time).find((part) => part.type === "dayPeriod");
        if (period === undefined) {
            throw new TypeError(`the clock of ${this.#locale.tag} writes no day period`);
        }

        return period.value;
    }

    /** Write a segment's value as it shows. */
    valueText(kind: DateSegmentKind, value: number): string {
        return this.text(kind, value);
    }

    /** Label a segment by the part of the time it holds. */
    name(kind: DateSegmentKind): string {
        // the hour
        if (kind === "hour") {
            return this.#locale.render(t`Hour`);
        }
        // the minute
        else if (kind === "minute") {
            return this.#locale.render(t`Minute`);
        }

        return this.#locale.render(t`AM/PM`);
    }

    /** Read the local clock's hour, minute or day period. */
    now(kind: DateSegmentKind): number {
        // read the clock's hour on the locale's hour cycle
        const now = new Date();
        const hours = now.getHours();
        if (kind === "hour") {
            return this.isTwelveHour ? hours % HALF_DAY || HALF_DAY : hours;
        }
        // the minute
        else if (kind === "minute") {
            return now.getMinutes();
        }

        return hours < HALF_DAY ? 0 : 1;
    }

    /** Read the time of the segments, undefined while one is empty. */
    timeOf(values: DateSegmentValues): PlainTime | undefined {
        // wait for every segment the locale shows
        const { hour, minute, dayPeriod } = values;
        const isPeriodMissing = this.isTwelveHour && dayPeriod === undefined;
        if (hour === undefined || minute === undefined || isPeriodMissing) {
            return undefined;
        }

        // count the hours of a 12-hour clock from midnight
        const hours = dayPeriod === undefined ? hour : (hour % HALF_DAY) + HALF_DAY * dayPeriod;

        return `${pad(hours)}:${pad(minute)}`;
    }

    /** Split a time into the values of its segments on the locale's hour cycle, none for no time. */
    valuesOf(time: PlainTime | undefined): DateSegmentValues {
        // leave every segment empty without a time
        if (time === undefined) {
            return {};
        }

        // split the hours into a 12-hour clock's hour and day period
        const minutes = PlainTime.minutes(time);
        const hours = Math.floor(minutes / HOUR_MINUTES);
        const minute = minutes % HOUR_MINUTES;

        return this.isTwelveHour
            ? { hour: hours % HALF_DAY || HALF_DAY, minute, dayPeriod: hours < HALF_DAY ? 0 : 1 }
            : { hour: hours, minute };
    }
}

/** Render a time of day as one spinbutton per hour, minute and day period in the locale's order and hour cycle. */
export function TimeField(properties: TimeFieldProperties): JSX.Element {
    // follow the controlled time, else the field's own, and the segments typed so far
    const format = new TimeFormat(useLocale());
    const rest = omit(properties, "value", "defaultValue", "onValueChange");
    const [value, setValue] = createControllableSignal<PlainTime | undefined>({
        isControlled: () => "value" in properties,
        value: () => properties.value,
        defaultValue: properties.defaultValue,
        onChange: (next) => properties.onValueChange?.(next),
    });
    const [values, setValues] = createSignal(format.valuesOf(untrack(value)), {
        ownedWrite: true,
    });

    // show a time set from outside, and clear the segments once a shown time is unset
    createEffect(
        value,
        (next, previous) => {
            const held = format.timeOf(untrack(values));
            if (next !== undefined && next !== held) {
                setValues(format.valuesOf(next));
            } else if (next === undefined && previous !== undefined && previous === held) {
                setValues({});
            }
        },
        { defer: true },
    );

    return (
        <DateSegmentGroup
            data-slot="time-field"
            {...rest}
            format={format}
            values={values()}
            onValuesChange={(next, isTyping) => {
                // keep the segments and report the time once it is complete or gone, typing done
                setValues(next);
                const time = format.timeOf(next);
                if (!isTyping && time !== value()) {
                    setValue(time);
                }
            }}
        />
    );
}

/** Write a part of a time in two digits. */
function pad(part: number): string {
    return String(part).padStart(2, "0");
}
