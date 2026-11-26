import { state } from "@destack/theme/state";
import * as style from "@destack/style";
import {
    color,
    motion,
    radius,
    size,
    space,
    stroke,
    weight,
    swatch,
} from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX, omit } from "@destack/view";
import { readText, takeText, useFieldControl, useFieldValue } from "../field/control.ts";
import { useJoin } from "../join/index.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import { defineComponent } from "@destack/view/declare";
import { t } from "@destack/locale";
import { field } from "@destack/object";
import { schema } from "@destack/schema";

/** The looks of an input: framed, or plain as a title is. */
const INPUT_VARIANTS = {
    outline: { title: t.context("option")`Outline`, description: t`A framed field, as in a form.` },
    plain: {
        title: t.context("option")`Plain`,
        description: t`A field without a frame, as in a toolbar or a search bar.`,
    },
};

/** The styles of an input and of every control that looks like one. */
const styles = style.create({
    box: {
        paddingInline: space[3],
        borderWidth: stroke.border,
        borderColor: { default: color.input, ":focus-visible": color.ring },
        borderRadius: radius[3],
        outlineOffset: `calc(-1 * ${stroke.border})`,
        color: color.foreground,
        transitionProperty: "border-color, outline-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        "::placeholder": { color: color.mutedForeground },
    },
    // leave the frame to what is around it, or to the caret alone
    plain: {
        paddingInline: 0,
        borderWidth: 0,
        borderRadius: 0,
        outlineStyle: "none",
        backgroundColor: "transparent",
        boxShadow: "none",
    },
    input: {
        width: "100%",
        minWidth: 0,
        height: size[2],
        paddingBlock: space[1],
        cursor: { default: "text", ":disabled": "not-allowed" },
        "::selection": { backgroundColor: swatch.accentSolid, color: swatch.accentLabel },
        "::file-selector-button": {
            height: "100%",
            marginInlineEnd: space[2],
            color: color.foreground,
            fontWeight: weight.medium,
        },
    },
});

/** The look of an input: its own frame, or none. */
export type InputVariant = keyof typeof INPUT_VARIANTS;

/** The look and state of an input's styles. */
export interface InputStyleOptions {
    /** The look, outline by default. */
    readonly variant?: InputVariant | undefined;
    /** Whether the value is refused, which marks the box destructive. */
    readonly invalid?: boolean;
}

/** Return the StyleX styles of an input's box. */
export function inputStyle(options: InputStyleOptions = {}): style.Styles {
    return [
        text.sm,
        state.typed,
        state.focus,
        state.disabled,
        styles.box,
        options.variant === "plain" && styles.plain,
        options.invalid === true && state.invalid,
    ];
}

/** The properties of an input, the native input's attributes included. */
export type InputProperties = Omit<JSX.InputHTMLAttributes<HTMLInputElement>, "class"> &
    ElementPartProperties & {
        /** The look, outline by default. */
        readonly variant?: InputVariant;
    };

/** Render a native input, tied to the label, descriptions and state of its field. */
export const Input = defineComponent(
    "input",
    function Input(given: InputProperties): JSX.Element {
        // edit the value of the field's form
        const properties = useFieldValue(given, {
            value: "value",
            change: "onInput",
            read: readText,
            take: takeText,
        });

        // take the id, descriptions and state of the nearest fieldControl
        const fieldControl = useFieldControl();
        const join = useJoin();
        const isInvalid = (): boolean =>
            fieldControl?.isInvalid() === true || properties["aria-invalid"] === "true";

        return renderPart(
            "input",
            "input",
            omit(properties, "variant"),
            () => [
                inputStyle({ variant: properties.variant, invalid: isInvalid() }),
                styles.input,
                join(),
            ],
            fieldControl?.attributes(),
        );
    },
    {
        variant: field
            .enum(INPUT_VARIANTS)
            .labeled(t.context("label")`Variant`)
            .described(t`Whether the field shows its frame.`)
            .default("outline"),
        placeholder: field
            .string(schema.string())
            .labeled(t.context("label")`Placeholder`)
            .described(t`The hint the field shows while it is empty.`)
            .optional(),
        disabled: field
            .boolean()
            .labeled(t.context("label")`Disabled`)
            .described(t`Refuses input where the value cannot change now.`)
            .default(false),
        readonly: field
            .boolean()
            .labeled(t.context("label")`Read only`)
            .described(t`Shows the value, which a person can select but not change.`)
            .default(false),
        required: field
            .boolean()
            .labeled(t.context("label")`Required`)
            .described(t`Asks for a value before its form submits.`)
            .default(false),
    },
);
