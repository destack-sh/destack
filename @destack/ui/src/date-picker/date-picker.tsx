import { Icon } from "@destack/icon";
import { type Localization, plural, t } from "@destack/locale";
import { PlainDate } from "@destack/schema";
import * as style from "@destack/style";
import { color, size, space, stroke, weight } from "@destack/theme/tokens.stylex";
import {
    createControllableSignal,
    createUniqueId,
    For,
    type JSX,
    Show,
    useLocale,
} from "@destack/view";
import { Button, type ButtonVariant } from "../button/index.ts";
import {
    Calendar,
    Day,
    isDays,
    isRange,
    type CalendarMultiple,
    type CalendarRange,
    type CalendarSelection,
    type CalendarSingle,
    type CalendarValue,
    type DateRange,
} from "../calendar/index.ts";
import { DateField } from "../date-field/index.ts";
import { FieldContext, useFieldControl } from "../field/control.ts";
import { Popover, PopoverContent, PopoverTrigger, usePopover } from "../popover/index.ts";
import { bareDateSegmentStyle, dateSegmentBoxStyle } from "../date-segment/index.ts";

/** The styles of a date picker's fields, trigger and panel. */
const styles = style.create({
    group: {
        gap: space[1],
        paddingInlineEnd: space[1],
    },
    separator: {
        color: color.mutedForeground,
    },
    button: {
        marginInlineStart: "auto",
    },
    trigger: {
        justifyContent: "flex-start",
        minWidth: `calc(6 * ${size[3]})`,
        fontWeight: weight.regular,
    },
    placeholder: {
        color: color.mutedForeground,
    },
    content: {
        display: "flex",
        width: "auto",
        padding: 0,
    },
    presets: {
        display: "flex",
        flexDirection: "column",
        gap: space[1],
        padding: space[3],
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: stroke.border,
        borderInlineEndColor: color.border,
    },
    preset: {
        justifyContent: "flex-start",
    },
});

/** The prompt, limits and look of a date picker in any mode. */
export interface DatePickerLook {
    /** The text a multiple date picker's trigger shows before a day is chosen, a generic prompt by default. */
    readonly placeholder?: string;
    /** Report whether a day cannot be chosen. */
    readonly isDisabled?: (day: PlainDate) => boolean;
    /** The look of a multiple date picker's trigger, outline by default. */
    readonly variant?: ButtonVariant;
    /** The name of the picker, such as the field it fills, which a multiple picker's chosen days follow. */
    readonly "aria-label"?: string;
    /** The choices offered beside the calendar, such as today or the next seven days. */
    readonly presets?: readonly DatePickerPreset[];
    /** Whether the calendar is open, which makes the open state controlled. */
    readonly open?: boolean | undefined;
    /** Whether the calendar starts open while the open state is uncontrolled. */
    readonly defaultOpen?: boolean | undefined;
    /** Handle the calendar opening or closing. */
    readonly onOpenChange?: ((open: boolean) => void) | undefined;
}

/** A choice a date picker offers beside its calendar. */
export interface DatePickerPreset {
    /** The choice's name, such as "Next 7 days". */
    readonly label: string;
    /** The days it chooses, in the picker's mode. */
    readonly value: CalendarValue;
}

/** A date picker typing or picking one day. */
export interface DatePickerSingle extends Omit<CalendarSingle, "onValueChange"> {
    /** Handle the day changing, undefined once the typed day is missing or impossible. */
    readonly onValueChange?: (value: PlainDate | undefined) => void;
}

/** A date picker typing or picking a range, its start first. */
export interface DatePickerRange extends Omit<CalendarRange, "onValueChange"> {
    /** Handle the range changing, undefined once the typed start is missing or impossible. */
    readonly onValueChange?: (value: DateRange | undefined) => void;
}

/** The selection of a date picker in any mode. */
export type DatePickerSelection = DatePickerSingle | DatePickerRange | CalendarMultiple;

/** The properties of a date picker: its look and the selection of its mode. */
export type DatePickerProperties = DatePickerLook & DatePickerSelection;

/**
 * Render a date picker: fields that type a day or a range beside a button opening a calendar.
 *
 * A multiple date picker opens its calendar from a button that shows the count of chosen days.
 */
export function DatePicker(properties: DatePickerProperties): JSX.Element {
    return (
        <Popover
            open={properties.open}
            defaultOpen={properties.defaultOpen}
            onOpenChange={properties.onOpenChange}
        >
            <DatePickerPanel {...properties} />
        </Popover>
    );
}

/** Render the fields or the trigger and the calendar of a date picker inside its popover. */
function DatePickerPanel(properties: DatePickerProperties): JSX.Element {
    // read the popover and the chosen days
    const popover = usePopover();
    const [value, setValue] = createControllableSignal<CalendarValue>({
        isControlled: () => "value" in properties,
        value: () => properties.value,
        defaultValue: properties.defaultValue,
        onChange: (next) => report(properties, next),
    });

    // keep and report a choice, closing the popover once it is complete
    const change = (next: CalendarValue, isComplete: boolean) => {
        setValue(next);
        if (isComplete) {
            popover.close();
        }
    };

    return (
        <>
            <Show
                when={properties.mode === "multiple"}
                fallback={
                    <DatePickerFields
                        properties={properties}
                        value={value()}
                        onValueChange={(next) => change(next, false)}
                    />
                }
            >
                <DatePickerTrigger properties={properties} value={value()} />
            </Show>
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
                xstyle={styles.content}
            >
                <Show when={properties.presets}>
                    {(presets) => (
                        <div
                            role="group"
                            data-slot="date-picker-presets"
                            {...style.attrs(styles.presets)}
                        >
                            <For each={presets()}>
                                {(preset) => (
                                    <Button
                                        variant="ghost"
                                        size="sm"
                                        xstyle={styles.preset}
                                        onClick={() => change(preset.value, true)}
                                    >
                                        {preset.label}
                                    </Button>
                                )}
                            </For>
                        </div>
                    )}
                </Show>
                <Calendar
                    {...selectionOf(properties.mode, value(), change)}
                    isDisabled={properties.isDisabled ?? (() => false)}
                />
            </PopoverContent>
        </>
    );
}

/** Render a group of the fields typing a day, or a range's start and end, and the button opening the calendar. */
function DatePickerFields(properties: {
    /** The date picker's properties. */
    readonly properties: DatePickerProperties;
    /** The chosen day or range. */
    readonly value: CalendarValue;
    /** Handle a typed day or range. */
    readonly onValueChange: (value: CalendarValue) => void;
}): JSX.Element {
    // label the group by its field's label, else by the picker's name
    const locale = useLocale();
    const field = useFieldControl();
    const picker = properties.properties;
    const range = (): DateRange | undefined =>
        isRange(properties.value) ? properties.value : undefined;
    field?.group();

    // keep the end typed last, which the range takes once a start before it is typed
    let end: PlainDate | undefined;

    return (
        <div
            role="group"
            data-slot="date-picker"
            aria-label={picker["aria-label"]}
            {...field?.groupAttributes()}
            {...style.attrs(
                dateSegmentBoxStyle({
                    invalid: field?.isInvalid() === true,
                    disabled: field?.isDisabled() === true,
                }),
                styles.group,
            )}
        >
            {/* Typed day, or typed start and end */}
            <FieldContext value={null}>
                <Show
                    when={picker.mode === "range"}
                    fallback={
                        <DateField
                            value={singleOf(properties.value)}
                            onValueChange={(day) => properties.onValueChange(day)}
                            xstyle={bareDateSegmentStyle()}
                        />
                    }
                >
                    <DateField
                        aria-label={locale.render(t`Start date`)}
                        value={range()?.from}
                        onValueChange={(from) =>
                            properties.onValueChange(rangeOf(from, range()?.to ?? end))
                        }
                        xstyle={bareDateSegmentStyle()}
                    />
                    <span aria-hidden="true" {...style.attrs(styles.separator)}>
                        –
                    </span>
                    <DateField
                        aria-label={locale.render(t`End date`)}
                        value={range()?.to}
                        onValueChange={(to) => {
                            // keep the typed end, reporting it once the range has a start
                            const from = range()?.from;
                            end = to;
                            if (from !== undefined) {
                                properties.onValueChange(rangeOf(from, to));
                            }
                        }}
                        xstyle={bareDateSegmentStyle()}
                    />
                </Show>
            </FieldContext>

            {/* Calendar button */}
            <PopoverTrigger
                variant="ghost"
                size="icon-sm"
                aria-label={locale.render(t`Pick a date`)}
                data-slot="date-picker-trigger"
                xstyle={styles.button}
            >
                <Icon name="calendar-blank" />
            </PopoverTrigger>
        </div>
    );
}

/** Render the button of a multiple date picker showing the chosen days, named by the picker's name followed by them. */
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
            xstyle={[styles.trigger, properties.value === undefined && styles.placeholder]}
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
                    isDays(properties.value) ? properties.value : [],
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
    mode: DatePickerSelection["mode"],
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
            value: singleOf(value),
            onValueChange: (day) => change(day, true),
        };
    }
}

/** Write the chosen days like the locale's medium dates, one as its date and several as a count. */
function labelOf(
    days: readonly PlainDate[],
    locale: Localization,
    placeholder: string,
    count: (days: number) => string,
): string {
    // prompt before a day is chosen
    const [first] = days;
    if (first === undefined) {
        return placeholder;
    }

    return days.length === 1
        ? locale.date(Day.time(first), { dateStyle: "medium", timeZone: "UTC" })
        : count(days.length);
}

/** Build the range from a start to an end, the end left out before the start, none without a start. */
function rangeOf(from: PlainDate | undefined, to: PlainDate | undefined): DateRange | undefined {
    // leave the range out without a start
    if (from === undefined) {
        return undefined;
    }

    return to !== undefined && PlainDate.compare(to, from) >= 0 ? { from, to } : { from };
}

/** Read the one day of a single date picker's value, undefined for none. */
function singleOf(value: CalendarValue): PlainDate | undefined {
    return value === undefined || isDays(value) || isRange(value) ? undefined : value;
}

/** Tell a date picker's change handler its new value, typed for its mode. */
function report(selection: DatePickerSelection, value: CalendarValue): void {
    // the chosen days
    if (selection.mode === "multiple") {
        selection.onValueChange?.(isDays(value) ? value : []);
    }
    // the range, none without a start
    else if (selection.mode === "range") {
        selection.onValueChange?.(isRange(value) ? value : undefined);
    }
    // the day, none once cleared
    else {
        selection.onValueChange?.(singleOf(value));
    }
}
