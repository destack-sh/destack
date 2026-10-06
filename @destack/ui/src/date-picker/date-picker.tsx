import { Icon } from "@destack/icon";
import { type Localization, plural, t } from "@destack/locale";
import type { PlainDate } from "@destack/schema";
import * as style from "@destack/style";
import { color, size, weight } from "@destack/theme/tokens.stylex";
import { useLocale } from "@destack/locale/solid";
import type { JSX } from "@solidjs/web";
import { createSignal, createUniqueId, Show } from "solid-js";
import type { ButtonVariant } from "../button/index.ts";
import {
    Calendar,
    Day,
    isDays,
    isRange,
    report,
    type CalendarSelection,
    type CalendarValue,
} from "../calendar/index.ts";
import { Popover, PopoverContent, PopoverTrigger, usePopover } from "../popover/index.ts";

/** The styles of a date picker's trigger and panel. */
const styles = style.create({
    trigger: {
        justifyContent: "flex-start",
        minWidth: `calc(6 * ${size[3]})`,
        fontWeight: weight.regular,
    },
    placeholder: {
        color: color.mutedForeground,
    },
    content: {
        width: "auto",
        padding: 0,
    },
});

/** The prompt, limits and look of a date picker in any mode. */
export interface DatePickerLook {
    /** The text the trigger shows before a day is chosen, a generic prompt by default. */
    readonly placeholder?: string;
    /** Report whether a day cannot be chosen. */
    readonly isDisabled?: (day: PlainDate) => boolean;
    /** The look of the trigger, outline by default. */
    readonly variant?: ButtonVariant;
    /** The name of the trigger, such as the field it fills, which the chosen days follow in its accessible name. */
    readonly "aria-label"?: string;
}

/** The properties of a date picker: its look and the selection of its calendar's mode. */
export type DatePickerProperties = DatePickerLook & CalendarSelection;

/** Render a button that opens a calendar in a popover, showing the chosen days in the locale's format. */
export function DatePicker(properties: DatePickerProperties): JSX.Element {
    return (
        <Popover>
            <DatePickerPanel {...properties} />
        </Popover>
    );
}

/** Render the trigger and the calendar of a date picker inside its popover. */
function DatePickerPanel(properties: DatePickerProperties): JSX.Element {
    // read the popover and the chosen days
    const popover = usePopover();
    const [ownValue, setValue] = createSignal<CalendarValue>(properties.defaultValue);
    const value = (): CalendarValue => ("value" in properties ? properties.value : ownValue());

    // keep and report a choice, closing the popover once it is complete
    const change = (next: CalendarValue, isComplete: boolean) => {
        setValue(next);
        report(properties, next);
        if (isComplete) {
            popover.close();
        }
    };

    return (
        <>
            <DatePickerTrigger properties={properties} value={value()} />
            <PopoverContent
                align="start"
                data-slot="date-picker-content"
                onToggle={(event) => {
                    // move the focus onto the calendar's focused day as it opens
                    if (event.newState === "open") {
                        event.currentTarget
                            .querySelector<HTMLElement>("[data-day][tabindex='0']")
                            ?.focus();
                    }
                }}
                style={styles.content}
            >
                <Calendar
                    {...selectionOf(properties.mode, value(), change)}
                    isDisabled={properties.isDisabled ?? (() => false)}
                />
            </PopoverContent>
        </>
    );
}

/** Render the button showing the chosen days, named by the date picker's name followed by them. */
function DatePickerTrigger(properties: {
    readonly properties: DatePickerProperties;
    readonly value: CalendarValue;
}): JSX.Element {
    // read the locale and the ids naming the button
    const locale = useLocale();
    const picker = properties.properties;
    const labelId = createUniqueId();
    const valueId = createUniqueId();

    return (
        <PopoverTrigger
            variant={picker.variant ?? "outline"}
            aria-labelledby={
                picker["aria-label"] === undefined ? undefined : `${labelId} ${valueId}`
            }
            aria-haspopup="dialog"
            data-slot="date-picker-trigger"
            style={[styles.trigger, properties.value === undefined && styles.placeholder]}
        >
            {/* Name and chosen days */}
            <Icon name="calendar-blank" />
            <Show when={picker["aria-label"]}>
                {(label) => (
                    <span id={labelId} hidden>
                        {label()}
                    </span>
                )}
            </Show>
            <span id={valueId}>
                {labelOf(
                    properties.value,
                    locale,
                    picker.placeholder ?? locale.render(t`Pick a date`),
                    (count) =>
                        locale.render(t`${plural(count, { one: "# date", other: "# dates" })}`),
                )}
            </span>
        </PopoverTrigger>
    );
}

/** Build the calendar selection of a mode, closing on a single day or a complete range. */
function selectionOf(
    mode: CalendarSelection["mode"],
    value: CalendarValue,
    change: (next: CalendarValue, isComplete: boolean) => void,
): CalendarSelection {
    if (mode === "multiple") {
        return {
            mode,
            value: isDays(value) ? value : [],
            onValueChange: (days) => change(days, false),
        };
    } else if (mode === "range") {
        return {
            mode,
            value: isRange(value) ? value : undefined,
            onValueChange: (range) => change(range, range.to !== undefined),
        };
    } else {
        return {
            mode: "single",
            value: value === undefined || isDays(value) || isRange(value) ? undefined : value,
            onValueChange: (day) => change(day, true),
        };
    }
}

/** Write the chosen days like the locale's medium dates, a range as one span and several as a count. */
function labelOf(
    value: CalendarValue,
    locale: Localization,
    placeholder: string,
    count: (days: number) => string,
): string {
    const medium = (day: PlainDate): string =>
        locale.date(Day.time(day), { dateStyle: "medium", timeZone: "UTC" });
    if (value === undefined || (isDays(value) && value.length === 0)) {
        return placeholder;
    } else if (isDays(value)) {
        return value.length === 1 && value[0] !== undefined
            ? medium(value[0])
            : count(value.length);
    } else if (isRange(value)) {
        return value.to === undefined
            ? medium(value.from)
            : locale.dateRange(Day.time(value.from), Day.time(value.to), {
                  dateStyle: "medium",
                  timeZone: "UTC",
              });
    } else {
        return medium(value);
    }
}
