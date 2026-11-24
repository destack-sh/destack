import { state } from "@destack/theme/state";
import { Icon } from "@destack/ui/icon";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { inputMarker } from "./marker.stylex.ts";
import { text } from "@destack/theme/text";
import { color, motion, radius, size, space, stroke } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    createSignal,
    type JSX,
    merge,
    omit,
    type Setter,
    Show,
    useContext,
} from "@destack/view";
import { readText, useFieldControl, useFieldValue } from "../field/control.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The characters a code takes when its owner allows no others. */
const DIGITS = "[0-9]";

/** The pattern of a code that sets none. */
const DEFAULTS: Required<Pick<OtpFieldProperties, "pattern">> = { pattern: DIGITS };

/** The blink of a slot's caret while it waits for a character. */
const blink = style.keyframes({
    "0%, 70%, 100%": { opacity: 1 },
    "20%, 50%": { opacity: 0 },
});

/** The styles of a one-time code input and its elements. */
const styles = style.create({
    root: {
        position: "relative",
        display: "flex",
        alignItems: "center",
        gap: space[2],
        width: "fit-content",
        opacity: { default: 1, [style.when.descendant(":disabled", inputMarker)]: 0.5 },
    },
    input: {
        position: "absolute",
        inset: 0,
        width: "100%",
        height: "100%",
        color: "transparent",
        caretColor: "transparent",
        fontSize: { default: "inherit", [media.pointerCoarse]: "max(1em, 16px)" },
        letterSpacing: "-0.5em",
        outlineStyle: "none",
        cursor: { default: "text", ":disabled": "not-allowed" },
    },
    group: {
        display: "flex",
        alignItems: "center",
    },
    slot: {
        position: "relative",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        width: size[3],
        height: size[3],
        borderColor: color.input,
        borderBlockWidth: stroke.border,
        borderInlineEndWidth: stroke.border,
        borderInlineStartWidth: { default: 0, ":first-child": stroke.border },
        borderStartStartRadius: { default: 0, ":first-child": radius[3] },
        borderEndStartRadius: { default: 0, ":first-child": radius[3] },
        borderStartEndRadius: { default: 0, ":last-child": radius[3] },
        borderEndEndRadius: { default: 0, ":last-child": radius[3] },
        transitionProperty: "border-color, outline-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    active: {
        zIndex: 1,
        borderColor: color.ring,
        outlineStyle: "solid",
        outlineWidth: stroke.ring,
        outlineOffset: `calc(-1 * ${stroke.border})`,
        outlineColor: color.ring,
    },
    caret: {
        width: stroke.border,
        height: "1em",
        backgroundColor: { default: color.foreground, [media.forcedColors]: "CanvasText" },
        animationName: blink,
        animationDuration: `calc(4 * ${motion.durationLong})`,
        animationIterationCount: "infinite",
    },
    separator: {
        display: "flex",
        color: color.mutedForeground,
    },
});

/** The code input of the nearest one-time code root, null outside one. */
const OtpFieldContext = createContext<OtpFieldControl | null>(null);

/** The typed code, focus and selection of a one-time code input, which its slots show. */
export class OtpFieldControl {
    /** The code as typed so far. */
    readonly value: Accessor<string>;
    /** Whether the input has the focus. */
    readonly isFocused: Accessor<boolean>;
    /** The start and end of the input's selection. */
    readonly selection: Accessor<readonly [number, number]>;
    /** Whether the code is invalid. */
    readonly isInvalid: Accessor<boolean>;
    /** The number of characters the code takes. */
    readonly maxLength: number;
    /** The properties of the root, read for its controlled value and change handlers. */
    readonly #properties: OtpFieldProperties;
    /** Replace the code and tell the change handler. */
    readonly #setValue: (value: string) => void;
    /** Replace whether the input has the focus. */
    readonly #setFocused: Setter<boolean>;
    /** Replace the input's selection. */
    readonly #setSelection: Setter<readonly [number, number]>;

    /** Create the state of a one-time code input from its properties. */
    constructor(properties: OtpFieldProperties, isInvalid: Accessor<boolean>) {
        // start from the default code, unfocused and with the caret at its end
        const [value, setValue] = createControllableSignal({
            isControlled: () => properties.value !== undefined,
            value: () => properties.value ?? "",
            defaultValue: properties.defaultValue ?? "",
            onChange: (next) => properties.onValueChange?.(next),
        });
        const [isFocused, setFocused] = createSignal(false);
        const [selection, setSelection] = createSignal<readonly [number, number]>([0, 0]);
        this.#properties = properties;
        this.value = value;
        this.isFocused = isFocused;
        this.selection = selection;
        this.isInvalid = isInvalid;
        this.maxLength = properties.maxLength;
        this.#setValue = setValue;
        this.#setFocused = setFocused;
        this.#setSelection = setSelection;
    }

    /** Return the character a slot shows, empty while the code is shorter. */
    charAt(index: number): string {
        return this.value()[index] ?? "";
    }

    /** Report whether a slot has the caret or a selected character while the input is focused. */
    isActive(index: number): boolean {
        // follow the caret, or the last slot once the code is complete
        const [start, end] = this.selection();
        const caret = Math.min(start, this.maxLength - 1);

        return (
            this.isFocused() && (start === end ? caret === index : start <= index && index < end)
        );
    }

    /** Take the text in the input. */
    input(element: HTMLInputElement): void {
        // take the allowed characters and put them back into the input
        const allowed = new RegExp(`^${this.#properties.pattern ?? DIGITS}$`, "u");
        const code = Array.from(element.value)
            .filter((character) => allowed.test(character))
            .join("")
            .slice(0, this.maxLength);
        element.value = code;
        this.select(element);

        // set the new code and tell the owner of it and of its completion
        this.#setValue(code);
        if (code.length === this.maxLength) {
            this.#properties.onComplete?.(code);
        }
    }

    /** Follow the input's selection. */
    select(element: HTMLInputElement): void {
        this.#setSelection([element.selectionStart ?? 0, element.selectionEnd ?? 0]);
    }

    /** Follow whether the input has the focus. */
    focus(element: HTMLInputElement, isFocused: boolean): void {
        this.#setFocused(isFocused);
        this.select(element);
    }
}

/** The properties of a one-time code input, the native input's attributes included. */
export interface OtpFieldProperties extends Omit<
    JSX.InputHTMLAttributes<HTMLInputElement>,
    "class" | "value" | "maxlength" | "pattern" | "children"
> {
    /** The number of characters the code takes. */
    readonly maxLength: number;
    /** The code, controlled by the owner. */
    readonly value?: string;
    /** The code at first when the owner leaves it uncontrolled. */
    readonly defaultValue?: string;
    /** The pattern each character must match, a digit by default. */
    readonly pattern?: string;
    /** Report the code each time it changes. */
    readonly onValueChange?: (value: string) => void;
    /** Report the code once all its characters are in. */
    readonly onComplete?: (value: string) => void;
    /** The slot groups and separators. */
    readonly children?: JSX.Element;
    /** The StyleX styles applied after the root's styles. */
    readonly xstyle?: style.Styles;
}

/** The properties of an element of a one-time code input. */
export type OtpFieldElementProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties;

/** The properties of a one-time code input's slot. */
export interface OtpFieldSlotProperties extends OtpFieldElementProperties {
    /** The position of the character the slot shows. */
    readonly index: number;
}

/** Render a one-time code input as one character per slot. */
export function OtpField(given: OtpFieldProperties): JSX.Element {
    // edit the value of the field's form
    const properties = useFieldValue(given, {
        value: "value",
        change: "onValueChange",
        read: readText,
    });

    // take the id, state and descriptions of the nearest field
    const field = useFieldControl();
    const code = merge(DEFAULTS, properties);
    const rest = omit(
        code,
        "maxLength",
        "value",
        "defaultValue",
        "pattern",
        "onValueChange",
        "onComplete",
        "children",
        "xstyle",
        "style",
    );
    const control = new OtpFieldControl(
        code,
        () => field?.isInvalid() === true || properties["aria-invalid"] === "true",
    );

    return (
        <OtpFieldContext value={control}>
            <div
                data-slot="otp-field"
                {...style.attributes([styles.root, code.xstyle], code.style)}
            >
                {code.children}
                <input
                    data-slot="otp-field-input"
                    autocomplete="one-time-code"
                    inputmode={code.pattern === DIGITS ? "numeric" : "text"}
                    spellcheck={false}
                    {...field?.attributes()}
                    {...rest}
                    maxlength={code.maxLength}
                    value={control.value()}
                    onInput={(event) => control.input(event.currentTarget)}
                    onSelect={(event) => control.select(event.currentTarget)}
                    onKeyUp={(event) => control.select(event.currentTarget)}
                    onFocus={(event) => control.focus(event.currentTarget, true)}
                    onBlur={(event) => control.focus(event.currentTarget, false)}
                    {...style.attrs(styles.input, inputMarker)}
                />
            </div>
        </OtpFieldContext>
    );
}

/** Render slots side by side, joined into one box. */
export function OtpFieldGroup(properties: OtpFieldElementProperties): JSX.Element {
    return renderPart("div", "otp-field-group", properties, styles.group);
}

/** Render one character of the code, with a blinking caret while it waits for it. */
export function OtpFieldSlot(properties: OtpFieldSlotProperties): JSX.Element {
    const control = useOtpField();
    const rest = omit(properties, "index", "xstyle", "style");

    return (
        <div
            data-slot="otp-field-slot"
            data-active={control.isActive(properties.index) ? "" : undefined}
            aria-hidden="true"
            {...rest}
            {...style.attributes(
                [
                    text.base,
                    styles.slot,
                    control.isActive(properties.index) && styles.active,
                    control.isInvalid() && state.invalid,
                    properties.xstyle,
                ],
                properties.style,
            )}
        >
            {control.charAt(properties.index)}
            <Show
                when={control.isActive(properties.index) && control.charAt(properties.index) === ""}
            >
                <div {...style.attrs(styles.caret)} />
            </Show>
        </div>
    );
}

/** Render a mark between groups of slots. */
export function OtpFieldSeparator(properties: OtpFieldElementProperties): JSX.Element {
    return renderPart("div", "otp-field-separator", properties, styles.separator, {
        role: "separator",
        get children() {
            return <Icon name="minus" />;
        },
    });
}

/** Read the control of the nearest one-time code input. */
export function useOtpField(): OtpFieldControl {
    const control = useContext(OtpFieldContext);
    if (control === null) {
        throw new TypeError("one-time code slots need a one-time code input around them");
    }

    return control;
}
