import { state } from "@destack/theme/state";
import type { Environment } from "@destack/environment";
import { type Localization, t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, motion, radius, size, space, stroke } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    createControllableSignal,
    createEffect,
    createSignal,
    For,
    type JSX,
    omit,
    Show,
    untrack,
    useEnvironment,
    useLocale,
} from "@destack/view";
import { type FieldControl, useFieldControl } from "../field/control.ts";
import { ListState } from "../focus/index.ts";
import { mergeProperties } from "../part/index.ts";
import { FormInput, type FormInputProperties, isFormControl } from "../form-input/index.ts";

/** The text an empty hour, minute or day period shows. */
const PLACEHOLDER_TIME = "––";

/** The kinds of the parts `formatToParts` lists that a segment edits. */
const KINDS: ReadonlySet<string> = new Set<DateSegmentKind>([
    "year",
    "month",
    "day",
    "hour",
    "minute",
    "dayPeriod",
]);

/** How far the page keys step each segment, as React Aria's. */
const PAGES: Readonly<Record<DateSegmentKind, number>> = {
    year: 5,
    month: 2,
    day: 7,
    hour: 2,
    minute: 15,
    dayPeriod: 1,
};

/** The keys that change a segment beside those that type. */
const CHANGES: ReadonlySet<string> = new Set([
    "ArrowUp",
    "ArrowDown",
    "PageUp",
    "PageDown",
    "Home",
    "End",
    "Backspace",
    "Delete",
]);

/** The styles of a segmented field and its segments. */
const styles = style.create({
    box: {
        position: "relative",
        display: "inline-flex",
        alignItems: "center",
        height: size[2],
        paddingInline: space[3],
        borderWidth: stroke.border,
        borderColor: { default: color.input, ":focus-within": color.ring },
        borderRadius: radius[3],
        color: color.foreground,
        fontVariantNumeric: "tabular-nums",
        whiteSpace: "nowrap",
        transitionProperty: "border-color, outline-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        outlineStyle: { default: "none", ":focus-within": "solid" },
        outlineWidth: stroke.ring,
        outlineOffset: `calc(-1 * ${stroke.border})`,
        outlineColor: color.ring,
    },
    bare: {
        height: "auto",
        paddingInline: 0,
        borderWidth: 0,
        boxShadow: "none",
        outlineStyle: "none",
    },
    segment: {
        paddingInline: `calc(${space[1]} / 4)`,
        borderRadius: radius[1],
        caretColor: "transparent",
        outlineStyle: "none",
        backgroundColor: {
            default: "transparent",
            ":focus": { default: color.selected, [media.forcedColors]: "Highlight" },
        },
        color: { default: "inherit", ":focus": color.foreground },
    },
    placeholder: {
        color: color.mutedForeground,
    },
    literal: {
        paddingInline: `calc(${space[1]} / 4)`,
        color: color.mutedForeground,
    },
});

/** The options of a box drawn around segments. */
export interface DateSegmentBoxOptions {
    /** Whether the value is refused, which marks the box destructive. */
    readonly invalid?: boolean;
    /** Whether the segments take no input, which dims the box. */
    readonly disabled?: boolean;
}

/** Return the StyleX styles of the input-like box around segments. */
export function dateSegmentBoxStyle(options: DateSegmentBoxOptions = {}): style.Styles {
    return [
        text.sm,
        styles.box,
        options.invalid === true && state.invalid,
        options.disabled === true && state.inert,
    ];
}

/** Return the StyleX styles of segments inside a box another control draws. */
export function bareDateSegmentStyle(): style.Styles {
    return styles.bare;
}

/** A part of a date or time a segment edits. */
export type DateSegmentKind = "year" | "month" | "day" | "hour" | "minute" | "dayPeriod";

/** The values of a segmented field's segments, absent while a segment is empty. */
export type DateSegmentValues = Partial<Readonly<Record<DateSegmentKind, number>>>;

/** How a segmented field lays out, bounds, writes and labels its segments in a locale. */
export interface DateSegmentLayout {
    /** The segments and the literals between them in the locale's order. */
    readonly layout: readonly Intl.DateTimeFormatPart[];
    /** Read the smallest and largest value of a segment, given the other segments. */
    readonly limits: (
        kind: DateSegmentKind,
        values: DateSegmentValues,
    ) => readonly [number, number];
    /** Write a segment's value as the field shows it. */
    readonly text: (kind: DateSegmentKind, value: number) => string;
    /** Write a segment's value for assistive technology. */
    readonly valueText: (kind: DateSegmentKind, value: number) => string;
    /** Label a segment for assistive technology. */
    readonly name: (kind: DateSegmentKind) => string;
    /** Read a segment's value now, where stepping an empty segment starts. */
    readonly now: (
        kind: DateSegmentKind,
        environment: Pick<Environment, "clock" | "timeZone">,
    ) => number;
}

/** How a segmented field reads a value of its segments, beside how it lays them out. */
export interface DateSegmentFormat<Value> extends DateSegmentLayout {
    /** Read the value of the segments, undefined while one is empty or there is none such. */
    readonly valueOf: (values: DateSegmentValues) => Value | undefined;
    /** Split a value into the values of its segments, none for no value. */
    readonly valuesOf: (value: Value | undefined) => DateSegmentValues;
    /** Order two values, zero for those that show the same segments. */
    readonly compare: (left: Value, right: Value) => number;
    /** Write a value as people read it. */
    readonly label: (value: Value) => string;
    /** Write a value as a form submits it. */
    readonly write: (value: Value) => string;
}

/** The properties of a field of segments, the native element's attributes included. */
export interface DateSegmentFieldProperties<Value>
    extends
        Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "children" | "onChange">,
        FormInputProperties {
    /** The value, which makes the field controlled, undefined while none is entered. */
    readonly value?: Value | undefined;
    /** The value the field starts from when uncontrolled. */
    readonly defaultValue?: Value | undefined;
    /** Handle the value changing: a complete one, or undefined once a segment is cleared. */
    readonly onValueChange?: (value: Value | undefined) => void;
    /** The earliest value, before which one is refused. */
    readonly min?: Value | undefined;
    /** The latest value, after which one is refused. */
    readonly max?: Value | undefined;
    /** Whether the field takes no input. */
    readonly disabled?: boolean | undefined;
    /** Whether the value is shown alone, its segments in reach of the keys. */
    readonly readOnly?: boolean | undefined;
    /** The StyleX styles applied after the field's styles. */
    readonly xstyle?: style.Styles;
}

/** The properties of a segmented field, the native element's attributes included. */
export interface DateSegmentGroupProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "children"
> {
    /** How the segments lay out, bound, read and name themselves. */
    readonly format: DateSegmentLayout;
    /** The values of the segments. */
    readonly values: DateSegmentValues;
    /** Handle a key that changes the segments. */
    readonly onValuesChange: (values: DateSegmentValues, isTyping: boolean) => void;
    /** Whether the segments take no input. */
    readonly disabled?: boolean | undefined;
    /** Whether the segments are shown alone. */
    readonly readOnly?: boolean | undefined;
    /** Whether the value is refused. */
    readonly invalid?: boolean | undefined;
    /** What submits the value with a form, none by default. */
    readonly input?: JSX.Element;
    /** The StyleX styles applied after the group's styles. */
    readonly xstyle?: style.Styles;
}

/** Report whether a part of a formatted date or time is a segment to edit. */
export function isDateSegment(
    part: Intl.DateTimeFormatPart,
): part is Intl.DateTimeFormatPart & { readonly type: DateSegmentKind } {
    return KINDS.has(part.type);
}

/** The digits typed into a segmented field's focused segment. */
export class DateSegmentControl {
    /** The group's properties, read for its format, values and change handler. */
    readonly properties: DateSegmentGroupProperties;
    /** The field the group belongs to, null outside one. */
    readonly field: FieldControl | null;
    /** The reader's locale, which sets the arrow keys' direction and the day periods' letters. */
    readonly #locale: Localization;
    /** The segments in document order, which the arrow keys and finished segments move between. */
    readonly list: ListState;
    /** The digits typed into the focused segment so far, empty once it takes no more. */
    #typed = "";
    /** The page's environment, whose clock and time zone an empty segment steps from. */
    readonly #environment: Pick<Environment, "clock" | "timeZone">;

    /** Create the state of a segmented field from its group's properties. */
    constructor(
        properties: DateSegmentGroupProperties,
        field: FieldControl | null,
        locale: Localization,
    ) {
        // set the properties and field, and the segments the arrow keys move between
        this.properties = properties;
        this.field = field;
        this.list = new ListState({
            orientation: "horizontal",
            isLooping: false,
            isTypeahead: false,
        });
        this.#locale = locale;
        this.#environment = useEnvironment();
    }

    /** Report whether the segments take no input. */
    isDisabled(): boolean {
        return this.properties.disabled === true || this.field?.isDisabled() === true;
    }

    /** Take a key a segment handles, reporting whether it did so the platform skips it. */
    press(key: string, kind: DateSegmentKind): boolean {
        // read the arrows of the writing direction
        const forward = this.#locale.direction === "rtl" ? "ArrowLeft" : "ArrowRight";
        const backward = this.#locale.direction === "rtl" ? "ArrowRight" : "ArrowLeft";
        const digit = key.length === 1 ? this.#locale.parseNumber(key) : undefined;

        // move between segments along the writing direction
        if (key === forward || key === backward) {
            this.#move(kind, key === forward ? 1 : -1);
        }
        // leave the value of segments shown alone
        else if (this.properties.readOnly === true) {
            return key.length === 1 || CHANGES.has(key);
        }
        // step the value, by more at once, or to its bounds
        else if (key === "ArrowUp" || key === "ArrowDown") {
            this.#step(kind, key === "ArrowUp" ? 1 : -1);
        } else if (key === "PageUp" || key === "PageDown") {
            this.#step(kind, key === "PageUp" ? PAGES[kind] : -PAGES[kind]);
        } else if (key === "Home" || key === "End") {
            this.#toLimit(kind, key === "Home" ? 0 : 1);
        }
        // clear the segment
        else if (key === "Backspace" || key === "Delete") {
            this.#typed = "";
            this.#change(kind, undefined);
        }
        // type a digit, of the locale's own too
        else if (digit !== undefined && kind !== "dayPeriod") {
            this.#type(String(digit), kind);
        }
        // choose the day period whose text starts with the typed letter
        else if (kind === "dayPeriod" && key.length === 1) {
            this.#choosePeriod(key);
        }
        // leave every other key, such as Tab, to the platform
        else {
            return false;
        }

        return true;
    }

    /** Finish typing a segment the focus leaves. */
    finish(): void {
        if (this.#typed !== "") {
            this.#typed = "";
            this.properties.onValuesChange(this.properties.values, false);
        }
    }

    /** Focus the segment beside one, the next along the writing direction or the previous. */
    #move(from: DateSegmentKind, step: 1 | -1): void {
        const next = step === 1 ? this.list.delegate.after(from) : this.list.delegate.before(from);
        if (next !== undefined) {
            this.list.focus.enter(next);
        }
    }

    /** Set a segment to its smallest or largest value. */
    #toLimit(kind: DateSegmentKind, place: 0 | 1): void {
        this.#typed = "";
        this.#change(kind, this.properties.format.limits(kind, this.properties.values)[place]);
    }

    /** Step a segment's value and wrap at its bounds, starting an empty segment from now. */
    #step(kind: DateSegmentKind, step: number): void {
        // step from the value, or from now while the segment is empty
        const [min, max] = this.properties.format.limits(kind, this.properties.values);
        const value = this.properties.values[kind];
        const stepped =
            value === undefined
                ? this.properties.format.now(kind, this.#environment)
                : value + step;
        this.#typed = "";
        this.#change(kind, wrap(stepped, min, max));
    }

    /** Type a digit into a segment, moving on once no further digit fits. */
    #type(digit: string, kind: DateSegmentKind): void {
        // append the digit, or start over past the largest value
        const [, max] = this.properties.format.limits(kind, this.properties.values);
        const appended = `${this.#typed}${digit}`;
        const digits = Number(appended) > max ? digit : appended;
        const isFull = Number(digits) * 10 > max || digits.length >= String(max).length;
        this.#typed = isFull ? "" : digits;
        this.#change(kind, Number(digits));
        if (isFull) {
            this.#move(kind, 1);
        }
    }

    /** Choose the day period whose text starts with a letter, moving on once chosen. */
    #choosePeriod(key: string): void {
        // find the morning or the afternoon by its first letter
        const format = this.properties.format;
        const letter = key.toLocaleLowerCase(this.#locale.tag);
        const period = format
            .limits("dayPeriod", this.properties.values)
            .find((entry) =>
                format
                    .text("dayPeriod", entry)
                    .toLocaleLowerCase(this.#locale.tag)
                    .startsWith(letter),
            );
        if (period !== undefined) {
            this.#change("dayPeriod", period);
            this.#move("dayPeriod", 1);
        }
    }

    /** Replace one segment's value. */
    #change(kind: DateSegmentKind, value: number | undefined): void {
        const { [kind]: _dropped, ...others } = this.properties.values;
        const next = value === undefined ? others : { ...others, [kind]: value };
        this.properties.onValuesChange(next, this.#typed !== "");
    }
}

/** Render the segments of a value, which follow the controlled value or the field's own. */
export function DateSegmentField<Value>(
    properties: DateSegmentFieldProperties<Value> & {
        /** How it reads and lays out its segments. */
        readonly format: DateSegmentFormat<Value>;
    },
): JSX.Element {
    // follow the value, and the segments typed so far
    const format = untrack(() => properties.format);
    const locale = useLocale();
    const field = useFieldControl();
    const rest = omit(
        properties,
        "format",
        "value",
        "defaultValue",
        "onValueChange",
        "min",
        "max",
        "name",
        "form",
        "required",
    );
    const [value, setValue] = createControllableSignal<Value | undefined>({
        isControlled: () => "value" in properties,
        value: () => properties.value,
        defaultValue: untrack(() => properties.defaultValue),
        onChange: (next) => properties.onValueChange?.(next),
    });
    const [values, setValues] = createSignal(format.valuesOf(untrack(value)), {
        ownedWrite: true,
    });
    const isSame = (left: Value | undefined, right: Value | undefined): boolean =>
        left === undefined || right === undefined
            ? left === right
            : format.compare(left, right) === 0;

    // show a value set from outside, and clear the segments once a shown value is unset
    createEffect(
        value,
        (next, previous) => {
            const current = format.valueOf(untrack(values));
            const isUnset = next === undefined && previous !== undefined;
            if (next !== undefined && !isSame(next, current)) {
                setValues(format.valuesOf(next));
            } else if (isUnset && isSame(previous, current)) {
                setValues({});
            }
        },
        { defer: true },
    );

    // refuse a value outside the limits, which stays shown
    const refusal = (): string => {
        // compare it with both
        const current = value();
        const { min, max } = properties;
        if (current !== undefined && min !== undefined && format.compare(current, min) < 0) {
            return locale.render(t`Choose ${format.label(min)} or later.`);
        } else if (current !== undefined && max !== undefined && format.compare(current, max) > 0) {
            return locale.render(t`Choose ${format.label(max)} or earlier.`);
        }

        return "";
    };
    const writeOf = (current: Value | undefined): string =>
        current === undefined ? "" : format.write(current);
    let group: HTMLElement | undefined;
    const reset = (): void => {
        const first = properties.defaultValue;
        setValues(format.valuesOf(first));
        setValue(first);
    };

    return (
        <DateSegmentGroup
            {...mergeProperties({ ref: (element: HTMLElement) => (group = element) }, rest)}
            invalid={refusal() !== ""}
            format={format}
            values={values()}
            input={
                <Show when={isFormControl(properties) || refusal() !== ""}>
                    <FormInput
                        name={properties.name}
                        form={properties.form}
                        required={properties.required}
                        disabled={properties.disabled === true || field?.isDisabled() === true}
                        value={writeOf(value())}
                        error={refusal()}
                        onReset={reset}
                        onFocus={() =>
                            group?.querySelector<HTMLElement>("[data-slot=segment]")?.focus()
                        }
                    />
                </Show>
            }
            onValuesChange={(next, isTyping) => {
                // set the segments and report the value once it is complete or gone, typing done
                setValues(next);
                const changed = format.valueOf(next);
                if (!isTyping && !isSame(changed, value())) {
                    setValue(changed);
                }
            }}
        />
    );
}

/** Render a group of spinbuttons, one per part of a date or time. */
export function DateSegmentGroup(properties: DateSegmentGroupProperties): JSX.Element {
    // label the group by its field's label and collect the digits typed into the focused segment
    const field = useFieldControl();
    const locale = useLocale();
    const control = new DateSegmentControl(properties, field, locale);
    const rest = omit(
        properties,
        "format",
        "values",
        "onValuesChange",
        "disabled",
        "readOnly",
        "invalid",
        "input",
        "xstyle",
        "style",
    );
    field?.group();
    const isInvalid = (): boolean =>
        properties.invalid === true ||
        field?.isInvalid() === true ||
        properties["aria-invalid"] === "true";

    return (
        <div
            role="group"
            {...field?.groupAttributes()}
            {...rest}
            aria-invalid={isInvalid() ? "true" : undefined}
            onFocusOut={(event) => control.list.focus.focusOut(event)}
            {...style.attributes(
                [
                    dateSegmentBoxStyle({
                        invalid: isInvalid(),
                        disabled: control.isDisabled(),
                    }),
                    properties.xstyle,
                ],
                properties.style,
            )}
        >
            <For each={properties.format.layout}>
                {(part) => (
                    <Show
                        when={isDateSegment(part) ? part.type : undefined}
                        fallback={
                            <span
                                aria-hidden="true"
                                data-slot="segment-literal"
                                {...style.attrs(styles.literal)}
                            >
                                {part.value}
                            </span>
                        }
                    >
                        {(kind) => (
                            <DateSegment
                                list={control.list}
                                kind={kind()}
                                format={properties.format}
                                values={properties.values}
                                isDisabled={control.isDisabled()}
                                isReadOnly={properties.readOnly === true}
                                emptyText={locale.render(t`Empty`)}
                                onKey={(key) => !control.isDisabled() && control.press(key, kind())}
                                onBlur={() => control.finish()}
                            />
                        )}
                    </Show>
                )}
            </For>
            {properties.input}
        </div>
    );
}

/** Write what an empty segment shows: the letters of a date's part, or dashes for a time's. */
function placeholderOf(kind: DateSegmentKind, locale: Localization): string {
    if (kind === "year") {
        return locale.render(t`yyyy`);
    } else if (kind === "month") {
        return locale.render(t`mm`);
    } else if (kind === "day") {
        return locale.render(t`dd`);
    } else {
        return PLACEHOLDER_TIME;
    }
}

/** Render one segment as a spinbutton named by its part, showing its value or a placeholder. */
function DateSegment(properties: {
    /** The segments of its group, which it joins. */
    readonly list: ListState;
    /** The part of the date or time it edits. */
    readonly kind: DateSegmentKind;
    /** How it bounds, writes and labels itself. */
    readonly format: DateSegmentLayout;
    /** The values of every segment, its own included. */
    readonly values: DateSegmentValues;
    /** Whether it takes no input. */
    readonly isDisabled: boolean;
    /** Whether it is shown alone. */
    readonly isReadOnly: boolean;
    /** The value text of an empty segment. */
    readonly emptyText: string;
    /** Handle a key typed into the segment, reporting whether it took it. */
    readonly onKey: (key: string) => boolean;
    /** Handle the focus leaving the segment. */
    readonly onBlur: () => void;
}): JSX.Element {
    // join the group's segments, which move the focus to this one by its kind
    const locale = useLocale();
    let element: HTMLElement | undefined;
    const kind = untrack(() => properties.kind);
    properties.list.add({
        key: kind,
        text: () => kind,
        isDisabled: () => false,
        element: () => element,
    });

    // read the value, its bounds and its texts, a placeholder while empty
    const value = (): number | undefined => properties.values[properties.kind];
    const isEmpty = (): boolean => value() === undefined;
    const limits = (): readonly [number, number] =>
        properties.format.limits(properties.kind, properties.values);
    const shown = (): string => {
        const current = value();

        return current === undefined
            ? placeholderOf(properties.kind, locale)
            : properties.format.text(properties.kind, current);
    };
    const spoken = (): string => {
        const current = value();

        return current === undefined
            ? properties.emptyText
            : properties.format.valueText(properties.kind, current);
    };

    return (
        <span
            role="spinbutton"
            tabindex={properties.isDisabled ? undefined : 0}
            contenteditable={properties.isDisabled || properties.isReadOnly ? undefined : "true"}
            inputmode={properties.kind === "dayPeriod" ? "text" : "numeric"}
            enterkeyhint="next"
            spellcheck={false}
            aria-label={properties.format.name(properties.kind)}
            aria-valuenow={value()}
            aria-valuemin={limits()[0]}
            aria-valuemax={limits()[1]}
            aria-valuetext={spoken()}
            aria-disabled={properties.isDisabled ? "true" : undefined}
            aria-readonly={properties.isReadOnly ? "true" : undefined}
            data-slot="segment"
            data-segment={properties.kind}
            data-placeholder={isEmpty() ? "" : undefined}
            onKeyDown={(event) => {
                // take the key without shortcut modifiers, before the platform does
                const isShortcut = event.altKey || event.ctrlKey || event.metaKey;
                if (!isShortcut && properties.onKey(event.key)) {
                    event.preventDefault();
                }
            }}
            ref={(segment) => (element = segment)}
            onFocus={() => properties.list.focus.focusIn(kind)}
            onBlur={() => properties.onBlur()}
            onBeforeInput={(event) => {
                // write the text as the field does and type a virtual keyboard's characters as keys
                event.preventDefault();
                for (const character of event.data ?? "") {
                    properties.onKey(character);
                }
            }}
            {...style.attrs(styles.segment, isEmpty() && styles.placeholder)}
        >
            {shown()}
        </span>
    );
}

/** Wrap a value into its bounds, past the largest to the smallest and back. */
function wrap(value: number, min: number, max: number): number {
    const span = max - min + 1;

    return ((((value - min) % span) + span) % span) + min;
}
