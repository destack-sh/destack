import { type Localization, t } from "@destack/locale";
import * as style from "@destack/style";
import { color, motion, radius, shadow, size, space, stroke } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { For, type JSX, omit, Show, untrack, useLocale } from "@destack/view";
import { type FieldControl, useFieldControl } from "../field/control.ts";
import { ListState } from "../focus/index.ts";

/** The text an empty segment shows in place of each digit. */
const PLACEHOLDER_DIGIT = "–";

/** The kinds of the parts `formatToParts` lists that a segment edits. */
const KINDS: ReadonlySet<string> = new Set<DateSegmentKind>([
    "year",
    "month",
    "day",
    "hour",
    "minute",
    "dayPeriod",
]);

/** The styles of a segmented field and its segments. */
const styles = style.create({
    box: {
        display: "inline-flex",
        alignItems: "center",
        boxSizing: "border-box",
        height: size[3],
        paddingInline: space[3],
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: { default: color.input, ":focus-within": color.ring },
        borderRadius: radius[3],
        backgroundColor: "transparent",
        boxShadow: shadow.inset,
        color: color.foreground,
        fontVariantNumeric: "tabular-nums",
        whiteSpace: "nowrap",
        transitionProperty: "border-color, outline-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        outlineStyle: { default: "none", ":focus-within": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
    },
    invalid: {
        borderColor: color.destructive,
        outlineColor: `color-mix(in oklab, ${color.destructive} 20%, transparent)`,
    },
    disabled: {
        opacity: 0.5,
        cursor: "not-allowed",
    },
    bare: {
        height: "auto",
        paddingInline: 0,
        borderWidth: 0,
        boxShadow: "none",
        outlineStyle: "none",
    },
    segment: {
        paddingInline: "1px",
        borderRadius: radius[1],
        caretColor: "transparent",
        outlineStyle: "none",
        backgroundColor: { default: "transparent", ":focus": color.accent },
        color: { default: "inherit", ":focus": color.accentForeground },
    },
    placeholder: {
        color: color.mutedForeground,
    },
    literal: {
        paddingInline: "1px",
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

/** Return the StyleX styles of the input-like box around segments, which date pickers also draw around their fields. */
export function dateSegmentBoxStyle(options: DateSegmentBoxOptions = {}): style.Styles {
    return [
        text.callout,
        styles.box,
        options.invalid === true && styles.invalid,
        options.disabled === true && styles.disabled,
    ];
}

/** Return the StyleX styles of segments inside a box another control draws, such as a date picker's. */
export function bareDateSegmentStyle(): style.Styles {
    return styles.bare;
}

/** A part of a date or time a segment edits. */
export type DateSegmentKind = "year" | "month" | "day" | "hour" | "minute" | "dayPeriod";

/** The values of a segmented field's segments, absent while a segment is empty. */
export type DateSegmentValues = Partial<Readonly<Record<DateSegmentKind, number>>>;

/** How a segmented field lays out, bounds, writes and names its segments in a locale. */
export interface DateSegmentFormat {
    /** The segments and the literals between them in the locale's order, as `formatToParts` lists them. */
    readonly layout: readonly Intl.DateTimeFormatPart[];
    /** Read the smallest and largest value of a segment, given the other segments. */
    limits(kind: DateSegmentKind, values: DateSegmentValues): readonly [number, number];
    /** Write a segment's value as the field shows it. */
    text(kind: DateSegmentKind, value: number): string;
    /** Write a segment's value for assistive technology. */
    valueText(kind: DateSegmentKind, value: number): string;
    /** Label a segment for assistive technology. */
    name(kind: DateSegmentKind): string;
    /** Read a segment's value now, where stepping an empty segment starts. */
    now(kind: DateSegmentKind): number;
}

/** The properties of a segmented field, the native element's attributes included. */
export interface DateSegmentGroupProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "children"
> {
    /** How the segments lay out, bound, read and name themselves. */
    readonly format: DateSegmentFormat;
    /** The values of the segments. */
    readonly values: DateSegmentValues;
    /** Handle a key changing the segments, told whether the focused segment still takes more digits. */
    readonly onValuesChange: (values: DateSegmentValues, isTyping: boolean) => void;
    /** Whether the segments take no input. */
    readonly disabled?: boolean;
    /** The StyleX styles applied after the group's styles. */
    readonly xstyle?: style.Styles;
}

/** Report whether a part of a formatted date or time is a segment to edit. */
export function isDateSegment(
    part: Intl.DateTimeFormatPart,
): part is Intl.DateTimeFormatPart & { readonly type: DateSegmentKind } {
    return KINDS.has(part.type);
}

/** The digits typed into a segmented field's focused segment and the keys that edit its segments. */
class DateSegmentControl {
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

    /** Create the state of a segmented field from its group's properties. */
    constructor(
        properties: DateSegmentGroupProperties,
        field: FieldControl | null,
        locale: Localization,
    ) {
        // keep the properties and field, and the segments the arrow keys move between
        this.properties = properties;
        this.field = field;
        this.list = new ListState({
            orientation: "horizontal",
            isLooping: false,
            isTypeahead: false,
        });
        this.#locale = locale;
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

        // move between segments along the writing direction
        if (key === forward || key === backward) {
            this.#move(kind, key === forward ? 1 : -1);
        }
        // step the value
        else if (key === "ArrowUp" || key === "ArrowDown") {
            this.#step(kind, key === "ArrowUp" ? 1 : -1);
        }
        // clear the segment
        else if (key === "Backspace") {
            this.#typed = "";
            this.#change(kind, undefined);
        }
        // type a digit
        else if (/^\d$/u.test(key) && kind !== "dayPeriod") {
            this.#type(key, kind);
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

    /** Step a segment's value and wrap at its bounds, starting an empty segment from now. */
    #step(kind: DateSegmentKind, step: 1 | -1): void {
        // step from the value, or from now while the segment is empty
        const [min, max] = this.properties.format.limits(kind, this.properties.values);
        const value = this.properties.values[kind];
        const stepped = value === undefined ? this.properties.format.now(kind) : value + step;
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

    /** Replace one segment's value and tell the owner whether digits are still being typed into it. */
    #change(kind: DateSegmentKind, value: number | undefined): void {
        const { [kind]: _dropped, ...others } = this.properties.values;
        const next = value === undefined ? others : { ...others, [kind]: value };
        this.properties.onValuesChange(next, this.#typed !== "");
    }
}

/** Render a group of spinbuttons, one per part of a date or time, which digits fill and arrow keys step and move through. */
export function DateSegmentGroup(properties: DateSegmentGroupProperties): JSX.Element {
    // label the group by its field's label and keep the digits typed into the focused segment
    const field = useFieldControl();
    const locale = useLocale();
    const control = new DateSegmentControl(properties, field, locale);
    const rest = omit(
        properties,
        "format",
        "values",
        "onValuesChange",
        "disabled",
        "xstyle",
        "style",
    );
    field?.group();

    return (
        <div
            role="group"
            {...field?.groupAttributes()}
            {...rest}
            onFocusOut={(event) => control.list.focus.focusOut(event)}
            {...style.attributes(
                [
                    dateSegmentBoxStyle({
                        invalid:
                            field?.isInvalid() === true || properties["aria-invalid"] === "true",
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
                                emptyText={locale.render(t`Empty`)}
                                onKey={(key) => !control.isDisabled() && control.press(key, kind())}
                                onBlur={() => control.finish()}
                            />
                        )}
                    </Show>
                )}
            </For>
        </div>
    );
}

/** Render one segment as a spinbutton named by its part, showing its value or a placeholder. */
function DateSegment(properties: {
    /** The segments of its group, which it joins. */
    readonly list: ListState;
    /** The part of the date or time it edits. */
    readonly kind: DateSegmentKind;
    /** How it bounds, writes and names itself. */
    readonly format: DateSegmentFormat;
    /** The values of every segment, its own included. */
    readonly values: DateSegmentValues;
    /** Whether it takes no input. */
    readonly isDisabled: boolean;
    /** The value text of an empty segment. */
    readonly emptyText: string;
    /** Handle a key typed into the segment, reporting whether it took it. */
    readonly onKey: (key: string) => boolean;
    /** Handle the focus leaving the segment. */
    readonly onBlur: () => void;
}): JSX.Element {
    // join the group's segments, which move the focus to this one by its kind
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
            ? PLACEHOLDER_DIGIT.repeat(properties.kind === "year" ? 4 : 2)
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
            contenteditable={properties.isDisabled ? undefined : "true"}
            inputmode={properties.kind === "dayPeriod" ? "text" : "numeric"}
            enterkeyhint="next"
            spellcheck={false}
            aria-label={properties.format.name(properties.kind)}
            aria-valuenow={value()}
            aria-valuemin={limits()[0]}
            aria-valuemax={limits()[1]}
            aria-valuetext={spoken()}
            aria-disabled={properties.isDisabled ? "true" : undefined}
            data-slot="segment"
            data-segment={properties.kind}
            data-placeholder={isEmpty() ? "" : undefined}
            onKeyDown={(event) => {
                // take the key without shortcut modifiers, keeping it from the platform
                const isShortcut = event.altKey || event.ctrlKey || event.metaKey;
                if (!isShortcut && properties.onKey(event.key)) {
                    event.preventDefault();
                }
            }}
            ref={(segment) => (element = segment)}
            onFocus={() => properties.list.focus.focusIn(kind)}
            onBlur={() => properties.onBlur()}
            onBeforeInput={(event) => {
                // keep the text as the field writes it and type a virtual keyboard's characters as keys
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
