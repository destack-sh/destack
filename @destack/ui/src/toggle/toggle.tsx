import { state } from "@destack/theme/state";
import * as style from "@destack/style";
import { type Render, renderPart } from "../part/index.ts";
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
import { createControllableSignal, type JSX, omit } from "@destack/view";
import {
    type ComponentProperties,
    type RenderProperties,
    defineComponent,
} from "@destack/view/declare";
import { field } from "@destack/object";
import { t } from "@destack/locale";

/** The looks of a toggle, each with the case it suits. */
const VARIANTS = {
    default: {
        title: t.context("option")`Default`,
        description: t`A toggle among other controls, clear until it is on.`,
    },
    outline: {
        title: t.context("option")`Outline`,
        description: t`A toggle in a toolbar, framed so its edge shows while it is off.`,
    },
    segmented: {
        title: t.context("option")`Segmented`,
        description: t`One segment of a track, raised while it is on.`,
    },
};

/** The heights and paddings of a toggle. */
const SIZES = {
    sm: { title: t.context("option")`Small` },
    default: { title: t.context("option")`Default` },
    lg: { title: t.context("option")`Large` },
};

/** The styles every toggle shares. */
const styles = style.create({
    toggle: {
        display: "inline-flex",
        flexShrink: 0,
        alignItems: "center",
        justifyContent: "center",
        gap: space[2],
        borderWidth: stroke.border,
        borderColor: "transparent",
        borderRadius: radius[3],
        color: color.foreground,
        fontWeight: weight.medium,
        whiteSpace: "nowrap",
        cursor: "pointer",
        transitionProperty: "color, background-color, outline-color, transform",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    // mute an outlined one that is off, which its fill alone hardly tells
    off: {
        color: color.mutedForeground,
    },
    // raise a pressed segment off its track, as a card
    raised: {
        backgroundColor: color.card,
        boxShadow: shadow.raised,
    },
});

/** The border of each variant. */
const variants = style.create({
    default: {},
    outline: { borderColor: color.input },
    segmented: { flex: "1 1 0", minWidth: 0, borderRadius: radius[2] },
});

/** The height and padding of each size. */
const sizes = style.create({
    sm: { minWidth: size[1], height: size[1], gap: space[1], paddingInline: space[2] },
    default: { minWidth: size[2], height: size[2], paddingInline: space[2] },
    lg: { minWidth: size[3], height: size[3], paddingInline: space[3] },
});

/** The height of each size of a segment, inside its track's inset so the track has the size. */
const segments = style.create({
    sm: { height: `calc(${size[1]} - 2 * ${stroke.ring})` },
    default: { height: `calc(${size[2]} - 2 * ${stroke.ring})` },
    lg: { height: `calc(${size[3]} - 2 * ${stroke.ring})` },
});

/** The look of a toggle. */
export type ToggleVariant = keyof typeof VARIANTS;

/** The height and padding of a toggle. */
export type ToggleSize = keyof typeof SIZES;

/** The properties people change in a toggle's controls and inspector. */
const TOGGLE_PROPERTIES = {
    variant: field
        .enum(VARIANTS)
        .labeled(t.context("label")`Variant`)
        .described(t`How the toggle shows whether it is on.`)
        .default("default"),
    size: field
        .enum(SIZES)
        .labeled(t.context("label")`Size`)
        .described(t`How tall the toggle is, and how much room its label gets.`)
        .default("default"),
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(t`Refuses presses where the setting cannot change now.`)
        .default(false),
};

/** The look of a toggle: its variant, its size and whether it is on. */
export interface ToggleLook {
    /** The look. */
    readonly variant: ToggleVariant;
    /** The height and padding. */
    readonly size: ToggleSize;
    /** Whether the toggle is on. */
    readonly pressed: boolean;
}

/** The properties of a toggle besides its declared ones: its native attributes, state and part. */
export interface ToggleAttributes extends Omit<
    JSX.ButtonHTMLAttributes<HTMLButtonElement>,
    "class" | keyof typeof TOGGLE_PROPERTIES
> {
    /** Whether the toggle is on, its owner's state, the toggle's own when absent. */
    readonly pressed?: boolean;
    /** Whether the toggle starts on when its state is uncontrolled. */
    readonly defaultPressed?: boolean;
    /** Handle the toggle turning on or off. */
    readonly onPressedChange?: (pressed: boolean) => void;
    /** The StyleX styles applied after the toggle's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with the toggle's attributes, the native button by default. */
    readonly render?: Render;
}

/** Return the StyleX styles of a toggle in its look. */
function toggleVariants(look: ToggleLook): style.Styles {
    return [
        text.sm,
        state.focus,
        state.disabled,
        state.hover,
        state.press,
        styles.toggle,
        variants[look.variant],
        sizes[look.size],
        look.variant === "segmented" && segments[look.size],
        look.variant !== "default" && !look.pressed && styles.off,
        look.pressed && state.on,
        look.variant === "segmented" && look.pressed && styles.raised,
    ];
}

/** Render a native button that turns on and off, reporting its state through `aria-pressed`. */
export const Toggle = Object.assign(
    defineComponent({
        name: "toggle",
        properties: TOGGLE_PROPERTIES,
        render(
            properties: RenderProperties<ToggleAttributes, typeof TOGGLE_PROPERTIES>,
        ): JSX.Element {
            // follow the controlled state, else the toggle's own
            const rest = omit(
                properties,
                "variant",
                "size",
                "pressed",
                "defaultPressed",
                "onPressedChange",
            );
            const [isPressed, setPressed] = createControllableSignal({
                isControlled: () => properties.pressed !== undefined,
                value: () => properties.pressed === true,
                defaultValue: properties.defaultPressed === true,
                onChange: (isNext) => properties.onPressedChange?.(isNext),
            });

            // mark the part, report its state and flip it
            return renderPart(
                "button",
                "toggle",
                rest,
                () =>
                    toggleVariants({
                        variant: properties.variant,
                        size: properties.size,
                        pressed: isPressed(),
                    }),
                {
                    type: "button" as const,
                    get "data-pressed"() {
                        return isPressed() ? "" : undefined;
                    },
                    get "aria-pressed"() {
                        return isPressed() ? "true" : "false";
                    },
                    onClick: () => {
                        // flip the state and tell the change handler
                        setPressed(!isPressed());
                    },
                },
            );
        },
    }),
    {
        /** Read the styles of a toggle, for an element that only looks like one. */
        styles: (look: Partial<ToggleLook>): style.Styles =>
            toggleVariants({ ...Toggle.defaults, pressed: false, ...look }),
    },
);

/** The properties of a toggle, the native button's attributes included. */
export type ToggleProperties = ComponentProperties<typeof Toggle>;
