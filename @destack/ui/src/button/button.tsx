import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, motion, radius, size, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX, merge, omit, Show } from "@destack/view";
import { type PartAttributes, type Render, rendered } from "../part/index.ts";
import { useJoin } from "../join/index.ts";
import { Spinner } from "../spinner/index.ts";

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
        transitionProperty: "color, background-color, border-color, outline-color, opacity",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: color.ring,
        opacity: { default: 1, ":disabled": 0.5 },
        pointerEvents: { default: "auto", ":disabled": "none" },
    },
});

/** The colors of each variant. */
const variants = style.create({
    default: {
        backgroundColor: {
            default: color.primary,
            ":hover": {
                default: null,
                [media.hover]: `color-mix(in oklab, ${color.primary} 90%, transparent)`,
            },
        },
        color: color.primaryForeground,
    },
    destructive: {
        backgroundColor: {
            default: color.destructive,
            ":hover": {
                default: null,
                [media.hover]: `color-mix(in oklab, ${color.destructive} 90%, transparent)`,
            },
        },
        color: color.destructiveForeground,
        outlineColor: `color-mix(in oklab, ${color.destructive} 40%, transparent)`,
    },
    outline: {
        backgroundColor: {
            default: color.background,
            ":hover": { default: null, [media.hover]: color.accent },
        },
        color: {
            default: color.foreground,
            ":hover": { default: null, [media.hover]: color.accentForeground },
        },
        borderColor: color.input,
    },
    secondary: {
        backgroundColor: {
            default: color.secondary,
            ":hover": {
                default: null,
                [media.hover]: `color-mix(in oklab, ${color.secondary} 80%, transparent)`,
            },
        },
        color: color.secondaryForeground,
    },
    ghost: {
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.accent },
        },
        color: {
            default: color.foreground,
            ":hover": { default: null, [media.hover]: color.accentForeground },
        },
    },
    link: {
        backgroundColor: "transparent",
        color: color.primary,
        textDecoration: {
            default: "none",
            ":hover": { default: null, [media.hover]: "underline" },
        },
        textUnderlineOffset: "0.25em",
    },
});

/** The height and padding of each size. */
const sizes = style.create({
    default: { height: size[3], paddingInline: space[4] },
    xs: {
        height: size[1],
        gap: space[1],
        paddingInline: space[2],
    },
    sm: { height: size[2], gap: space[1], paddingInline: space[3] },
    lg: { height: size[4], paddingInline: space[5] },
    icon: { width: size[3], height: size[3] },
    "icon-xs": { width: size[1], height: size[1] },
    "icon-sm": { width: size[2], height: size[2] },
    "icon-lg": { width: size[4], height: size[4] },
});

/** The look of a button, from most to least prominent. */
export type ButtonVariant = "default" | "destructive" | "outline" | "secondary" | "ghost" | "link";

/** The sizes that set their text in the footnote style. */
const FOOTNOTE_SIZES: ReadonlySet<ButtonSize> = new Set(["xs", "icon-xs"]);

/** The height and padding of a button, with square sizes for a lone icon. */
export type ButtonSize =
    | "default"
    | "xs"
    | "sm"
    | "lg"
    | "icon"
    | "icon-xs"
    | "icon-sm"
    | "icon-lg";

/** The properties of a button, the native button's attributes included. */
export interface ButtonProperties
    extends Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "class">, ButtonStyleOptions {
    /** The part's name for styling, `button` by default, which a part built on a button sets to its own. */
    readonly "data-slot"?: string;
    /** The StyleX styles applied after the button's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with the button's attributes, such as a link, the native button by default. */
    readonly render?: Render;
    /** Whether the button's action runs, which shows a spinner, marks it busy and disables it. */
    readonly loading?: boolean;
}

/** The variant and size of a button's styles. */
export interface ButtonStyleOptions {
    /** The look, default by default. */
    readonly variant?: ButtonVariant;
    /** The height and padding, default by default. */
    readonly size?: ButtonSize;
}

/** Return the StyleX styles of a button in a variant and size, for links and other elements that look like buttons. */
export function buttonStyle(options: ButtonStyleOptions): style.Styles {
    const height = options.size ?? DEFAULTS.size;

    return [
        FOOTNOTE_SIZES.has(height) ? text.footnote : text.callout,
        styles.button,
        variants[options.variant ?? DEFAULTS.variant],
        sizes[height],
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
        () => style.attributes([buttonStyle(button), join(), button.xstyle], button.style),
    );

    return rendered(button.render, part, rest, () => <button {...part} {...rest} />);
}
