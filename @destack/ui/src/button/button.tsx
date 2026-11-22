import { state } from "@destack/theme/state";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
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
import { type JSX, merge, omit, Show, untrack } from "@destack/view";
import { type PartAttributes, type Render, useRender } from "../part/index.ts";
import { useJoin } from "../join/index.ts";
import { Spinner } from "../spinner/index.ts";

/** The sizes that set their text in the extra small style. */
const SMALL_SIZES: ReadonlySet<ButtonSize> = new Set(["sm", "icon-sm"]);

/** The variants that are clear at rest and fill under the pointer. */
const HOVER_VARIANTS: ReadonlySet<ButtonVariant> = new Set(["outline", "ghost"]);

/** The variant and size of a button that sets neither. */
const DEFAULTS: Required<Pick<ButtonProperties, "variant" | "size">> = {
    variant: "default",
    size: "default",
};

/** The styles every button shares. */
const styles = style.create({
    button: {
        display: "inline-flex",
        flexShrink: 0,
        alignItems: "center",
        justifyContent: "center",
        gap: space[2],
        borderWidth: stroke.border,
        borderColor: "transparent",
        borderRadius: radius[3],
        fontWeight: weight.medium,
        whiteSpace: "nowrap",
        cursor: "pointer",
        transitionProperty:
            "color, background-color, border-color, outline-color, opacity, transform",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    flush: {
        paddingInline: 0,
    },
});

/** The colors of each variant. */
const variants = style.create({
    default: {
        backgroundColor: {
            default: swatch.accentSolid,
            ":hover": {
                default: null,
                [media.hover]: `color-mix(in oklab, ${swatch.accentSolid} 90%, transparent)`,
            },
        },
        color: swatch.accentLabel,
    },
    destructive: {
        backgroundColor: {
            default: swatch.destructiveSolid,
            ":hover": {
                default: null,
                [media.hover]: `color-mix(in oklab, ${swatch.destructiveSolid} 90%, transparent)`,
            },
        },
        color: swatch.destructiveLabel,
        outlineColor: `color-mix(in oklab, ${swatch.destructiveSolid} 40%, transparent)`,
    },
    outline: {
        color: color.foreground,
        borderColor: color.input,
    },
    secondary: {
        backgroundColor: {
            default: color.muted,
            ":hover": {
                default: null,
                [media.hover]: `color-mix(in oklab, ${color.muted} 80%, transparent)`,
            },
        },
        color: color.foreground,
    },
    ghost: {
        color: color.foreground,
    },
    link: {
        backgroundColor: "transparent",
        color: swatch.accentText,
        textDecoration: {
            default: "none",
            ":hover": { default: null, [media.hover]: "underline" },
        },
        textUnderlineOffset: "0.25em",
    },
});

/** The height and padding of each size. */
const sizes = style.create({
    inline: { height: "auto", paddingInline: 0, verticalAlign: "baseline" },
    sm: { height: size[1], gap: space[1], paddingInline: space[2] },
    default: { height: size[2], paddingInline: space[3] },
    lg: { height: size[3], paddingInline: space[4] },
    "icon-sm": { width: size[1], height: size[1] },
    icon: { width: size[2], height: size[2] },
    "icon-lg": { width: size[3], height: size[3] },
});

/** The look of a button, from most to least prominent. */
export type ButtonVariant = "default" | "destructive" | "outline" | "secondary" | "ghost" | "link";

/** The height and padding of a button, square for a lone icon, none inside running text. */
export type ButtonSize = "inline" | "sm" | "default" | "lg" | "icon-sm" | "icon" | "icon-lg";

/** The properties of a button, the native button's attributes included. */
export interface ButtonProperties
    extends Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "class">, ButtonVariantProperties {
    /** The part's name for styling, `button` by default. */
    readonly "data-slot"?: string;
    /** The StyleX styles applied after the button's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with the button's attributes, such as a link. */
    readonly render?: Render;
    /** Whether the button's action runs, which shows a spinner, marks it busy and disables it. */
    readonly loading?: boolean;
}

/** The variant and size of a button's styles. */
export interface ButtonVariantProperties {
    /** The look, default by default. */
    readonly variant?: ButtonVariant;
    /** The height and padding, default by default. */
    readonly size?: ButtonSize;
}

/** Return the StyleX styles of a button in a variant and size. */
export function buttonVariants(options: ButtonVariantProperties): style.Styles {
    const height = options.size ?? DEFAULTS.size;

    return [
        SMALL_SIZES.has(height) ? text.xs : text.sm,
        state.focus,
        state.disabled,
        state.press,
        styles.button,
        HOVER_VARIANTS.has(options.variant ?? DEFAULTS.variant) && state.hover,
        variants[options.variant ?? DEFAULTS.variant],
        sizes[height],
        options.variant === "link" && styles.flush,
    ];
}

/** Render a native button in a variant and size. */
export function Button(properties: ButtonProperties): JSX.Element {
    // join the button to its neighbours in a button group
    const join = useJoin();
    const button = merge(DEFAULTS, properties);
    const rest = omit(
        button,
        "variant",
        "size",
        "xstyle",
        "style",
        "render",
        "data-slot",
        "loading",
        "disabled",
        "children",
    );

    // mark the part and give it its styles with the caller's last
    const part: PartAttributes = merge(
        {
            get "data-slot"() {
                return button["data-slot"] ?? "button";
            },
            get "data-variant"() {
                return button.variant;
            },
            get "data-size"() {
                return button.size;
            },
            get "aria-busy"() {
                return button.loading === true ? "true" : undefined;
            },
            get disabled() {
                return button.disabled === true || button.loading === true ? true : undefined;
            },
            get children() {
                return (
                    <>
                        <Show when={button.loading}>
                            <Spinner aria-hidden="true" />
                        </Show>
                        {button.children}
                    </>
                );
            },
        },
        () => style.attributes([buttonVariants(button), join(), button.xstyle], button.style),
    );

    return useRender({
        render: untrack(() => button.render),
        defaultTagName: "button",
        part: part,
        properties: rest,
    });
}
