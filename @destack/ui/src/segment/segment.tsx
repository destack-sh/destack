import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, motion, radius, shadow, size, space, stroke } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { For, type JSX, omit, Show, useLocale } from "@destack/view";
import { useFieldControl } from "../field/control.ts";
import { itemsOf } from "../focus/index.ts";

/** The text an empty segment shows in place of each digit. */
const PLACEHOLDER_DIGIT = "–";

/** The kinds of the parts `formatToParts` lists that a segment edits. */
const KINDS: ReadonlySet<string> = new Set<SegmentKind>([
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
export interface SegmentBoxOptions {
    /** Whether the value is refused, which marks the box destructive. */
    readonly invalid?: boolean;
    /** Whether the segments take no input, which dims the box. */
    readonly disabled?: boolean;
}

/** Return the StyleX styles of the input-like box around segments, which date pickers also draw around their fields. */
export function segmentBoxStyle(options: SegmentBoxOptions = {}): style.Styles {
    return [
        text.callout,
        styles.box,
        options.invalid === true && styles.invalid,
        options.disabled === true && styles.disabled,
    ];
}

/** Return the StyleX styles of segments inside a box another control draws, such as a date picker's. */
export function bareSegmentStyle(): style.Styles {
    return styles.bare;
}

/** A part of a date or time a segment edits. */
export type SegmentKind = "year" | "month" | "day" | "hour" | "minute" | "dayPeriod";

/** The values of a segmented field's segments, absent while a segment is empty. */
export type SegmentValues = Partial<Readonly<Record<SegmentKind, number>>>;

/** How a segmented field lays out, bounds, writes and names its segments in a locale. */
export interface SegmentFormat {
    /** The segments and the literals between them in the locale's order, as `formatToParts` lists them. */
    readonly layout: readonly Intl.DateTimeFormatPart[];
    /** Read the smallest and largest value of a segment, given the other segments. */
    limits(kind: SegmentKind, values: SegmentValues): readonly [number, number];
    /** Write a segment's value as the field shows it. */
    text(kind: SegmentKind, value: number): string;
    /** Write a segment's value for assistive technology. */
    valueText(kind: SegmentKind, value: number): string;
    /** Name a segment for assistive technology. */
    name(kind: SegmentKind): string;
    /** Read a segment's value now, where stepping an empty segment starts. */
    now(kind: SegmentKind): number;
}

/** The properties of a segmented field, the native element's attributes included. */
export interface SegmentGroupProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "children"
> {
    /** How the segments lay out, bound, read and name themselves. */
    readonly format: SegmentFormat;
    /** The values of the segments. */
    readonly values: SegmentValues;
    /** Handle a key changing the segments, told whether the focused segment still takes more digits. */
    readonly onValuesChange: (values: SegmentValues, isTyping: boolean) => void;
    /** Whether the segments take no input. */
    readonly disabled?: boolean;
    /** The StyleX styles applied after the group's styles. */
    readonly xstyle?: style.Styles;
}

/** Report whether a part of a formatted date or time is a segment to edit. */
export function isSegment(
    part: Intl.DateTimeFormatPart,
): part is Intl.DateTimeFormatPart & { readonly type: SegmentKind } {
    return KINDS.has(part.type);
}

/** Render a group of spinbuttons, one per part of a date or time, which digits fill and arrow keys step and move through. */
export function SegmentGroup(properties: SegmentGroupProperties): JSX.Element {
    // name the group by its field's label and keep the digits typed into the focused segment
    const field = useFieldControl();
    const locale = useLocale();
    const rest = omit(
        properties,
        "format",
        "values",
        "onValuesChange",
        "disabled",
        "xstyle",
        "style",
    );
    const isDisabled = (): boolean => properties.disabled === true || field?.isDisabled() === true;
    let typed = "";
    let group: HTMLDivElement | undefined;
    field?.group();

    // focus the segment beside one, the next along the writing direction or the previous
    const move = (from: HTMLElement, step: number): void => {
        const segments = group === undefined ? [] : itemsOf(group, "[role=spinbutton]");
        segments[segments.indexOf(from) + step]?.focus();
    };

    // replace one segment's value and tell the owner whether digits are still being typed into it
    const change = (kind: SegmentKind, value: number | undefined): void => {
        const { [kind]: _dropped, ...others } = properties.values;
        const next = value === undefined ? others : { ...others, [kind]: value };
        properties.onValuesChange(next, typed !== "");
    };

    // take a key a segment handles, reporting whether it did so the platform skips it
    const press = (key: string, segment: HTMLElement, kind: SegmentKind): boolean => {
        // read the arrows of the writing direction and the segment's bounds
        const forward = locale.direction === "rtl" ? "ArrowLeft" : "ArrowRight";
        const backward = locale.direction === "rtl" ? "ArrowRight" : "ArrowLeft";
        const [min, max] = properties.format.limits(kind, properties.values);
        const value = properties.values[kind];

        // move between segments along the writing direction
        if (key === forward || key === backward) {
            move(segment, key === forward ? 1 : -1);
        }
        // step the value and wrap, starting an empty segment from now
        else if (key === "ArrowUp" || key === "ArrowDown") {
            const stepped =
                value === undefined
                    ? properties.format.now(kind)
                    : value + (key === "ArrowUp" ? 1 : -1);
            typed = "";
            change(kind, wrap(stepped, min, max));
        }
        // clear the segment
        else if (key === "Backspace") {
            typed = "";
            change(kind, undefined);
        }
        // type a digit, moving on once no further digit fits
        else if (/^\d$/u.test(key) && kind !== "dayPeriod") {
            const appended = `${typed}${key}`;
            const digits = Number(appended) > max ? key : appended;
            const isFull = Number(digits) * 10 > max || digits.length >= String(max).length;
            typed = isFull ? "" : digits;
            change(kind, Number(digits));
            if (isFull) {
                move(segment, 1);
            }
        }
        // choose the day period whose text starts with the typed letter
        else if (kind === "dayPeriod" && key.length === 1) {
            const letter = key.toLocaleLowerCase(locale.tag);
            const period = [min, max].find((entry) =>
                properties.format
                    .text(kind, entry)
                    .toLocaleLowerCase(locale.tag)
                    .startsWith(letter),
            );
            if (period !== undefined) {
                change(kind, period);
                move(segment, 1);
            }
        }
        // leave every other key, such as Tab, to the platform
        else {
            return false;
        }

        return true;
    };

    return (
        <div
            role="group"
            {...field?.groupAttributes()}
            {...rest}
            ref={(element) => {
                group = element;
            }}
            {...style.attributes(
                [
                    segmentBoxStyle({
                        invalid:
                            field?.isInvalid() === true || properties["aria-invalid"] === "true",
                        disabled: isDisabled(),
                    }),
                    properties.xstyle,
                ],
                properties.style,
            )}
        >
            <For each={properties.format.layout}>
                {(part) => (
                    <Show
                        when={isSegment(part) ? part.type : undefined}
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
                            <Segment
                                kind={kind()}
                                format={properties.format}
                                values={properties.values}
                                isDisabled={isDisabled()}
                                emptyText={locale.render(t`Empty`)}
                                onKey={(key, segment) =>
                                    !isDisabled() && press(key, segment, kind())
                                }
                                onBlur={() => {
                                    // finish typing a segment the focus leaves
                                    if (typed !== "") {
                                        typed = "";
                                        properties.onValuesChange(properties.values, false);
                                    }
                                }}
                            />
                        )}
                    </Show>
                )}
            </For>
        </div>
    );
}

/** Render one segment as a spinbutton named by its part, showing its value or a placeholder. */
function Segment(properties: {
    /** The part of the date or time it edits. */
    readonly kind: SegmentKind;
    /** How it bounds, writes and names itself. */
    readonly format: SegmentFormat;
    /** The values of every segment, its own included. */
    readonly values: SegmentValues;
    /** Whether it takes no input. */
    readonly isDisabled: boolean;
    /** The value text of an empty segment. */
    readonly emptyText: string;
    /** Handle a key typed into the segment, reporting whether it took it. */
    readonly onKey: (key: string, segment: HTMLElement) => boolean;
    /** Handle the focus leaving the segment. */
    readonly onBlur: () => void;
}): JSX.Element {
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
                if (!isShortcut && properties.onKey(event.key, event.currentTarget)) {
                    event.preventDefault();
                }
            }}
            onBlur={() => properties.onBlur()}
            onBeforeInput={(event) => {
                // keep the text as the field writes it and type a virtual keyboard's characters as keys
                event.preventDefault();
                for (const character of event.data ?? "") {
                    properties.onKey(character, event.currentTarget);
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
