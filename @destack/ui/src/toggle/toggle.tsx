import * as style from "@destack/style";
import { type PartAttributes, type Render, rendered } from "../part/index.ts";
import { media } from "@destack/style/media.stylex";
import {
    color,
    motion,
    radius,
    shadow,
    size,
    space,
    stroke,
    weight,
} from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { createControllableSignal, type JSX, merge, omit } from "@destack/view";

/** The styles every toggle shares. */
const styles = style.create({
    toggle: {
        display: "inline-flex",
        flexShrink: 0,
        alignItems: "center",
        justifyContent: "center",
        gap: space[2],
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: "transparent",
        borderRadius: radius[3],
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.muted },
        },
        color: {
            default: color.foreground,
            ":hover": { default: null, [media.hover]: color.mutedForeground },
        },
        fontWeight: weight.medium,
        whiteSpace: "nowrap",
        cursor: "pointer",
        transitionProperty: "color, background-color, outline-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
        opacity: { default: 1, ":disabled": 0.5 },
        pointerEvents: { default: "auto", ":disabled": "none" },
    },
    pressed: {
        backgroundColor: color.accent,
        color: color.accentForeground,
    },
});

/** The border of each variant. */
const variants = style.create({
    default: {},
    outline: { borderColor: color.input, boxShadow: shadow.inset },
});

/** The height and padding of each size. */
const sizes = style.create({
    default: { minWidth: size[3], height: size[3], paddingInline: space[2] },
    sm: { minWidth: size[2], height: size[2], paddingInline: space[2] },
    lg: { minWidth: size[4], height: size[4], paddingInline: space[3] },
});

/** The look of a toggle. */
export type ToggleVariant = "default" | "outline";

/** The height and padding of a toggle. */
export type ToggleSize = "default" | "sm" | "lg";

/** The variant, size and state of a toggle's styles. */
export interface ToggleStyleOptions {
    /** The look, default by default. */
    readonly variant?: ToggleVariant;
    /** The height and padding, default by default. */
    readonly size?: ToggleSize;
    /** Whether the toggle is on. */
    readonly pressed?: boolean;
}

/** The properties of a toggle, the native button's attributes included. */
export interface ToggleProperties
    extends
        Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "class" | "onClick">,
        ToggleStyleOptions {
    /** Whether the toggle starts on when its state is uncontrolled. */
    readonly defaultPressed?: boolean;
    /** Handle the toggle turning on or off. */
    readonly onPressedChange?: (pressed: boolean) => void;
    /** The StyleX styles applied after the toggle's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with the toggle's attributes, the native button by default. */
    readonly render?: Render;
}

/** Return the StyleX styles of a toggle in a variant, size and state, for toggle groups and other buttons that toggle. */
export function toggleStyle(options: ToggleStyleOptions): style.Styles {
    return [
        text.callout,
        styles.toggle,
        variants[options.variant ?? "default"],
        sizes[options.size ?? "default"],
        options.pressed === true && styles.pressed,
    ];
}

/** Render a native button that turns on and off, reporting its state through `aria-pressed`. */
export function Toggle(properties: ToggleProperties): JSX.Element {
    // follow the controlled state, else the toggle's own
    const rest = omit(
        properties,
        "variant",
        "size",
        "pressed",
        "defaultPressed",
        "onPressedChange",
        "xstyle",
        "style",
        "render",
    );
    const [isPressed, setPressed] = createControllableSignal({
        isControlled: () => properties.pressed !== undefined,
        value: () => properties.pressed === true,
        defaultValue: properties.defaultPressed === true,
        onChange: (isNext) => properties.onPressedChange?.(isNext),
    });

    // mark the part, report its state and flip it
    const part: PartAttributes = merge(
        {
            "data-slot": "toggle",
            get "data-state"() {
                return isPressed() ? "on" : "off";
            },
            get "aria-pressed"() {
                return isPressed() ? "true" : "false";
            },
            onClick: () => {
                // flip the state and tell the change handler
                setPressed(!isPressed());
            },
        },
        () =>
            style.attributes(
                [
                    toggleStyle({
                        variant: properties.variant ?? "default",
                        size: properties.size ?? "default",
                        pressed: isPressed(),
                    }),
                    properties.xstyle,
                ],
                properties.style,
            ),
    );

    return rendered(properties.render, part, rest, () => (
        <button type="button" {...part} {...rest} />
    ));
}
