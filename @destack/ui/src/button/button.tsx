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
import {
    createEffect,
    createSignal,
    type JSX,
    merge,
    omit,
    onCleanup,
    Show,
    untrack,
    useEnvironment,
} from "@destack/view";
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

/** How long an action runs before its button shows a spinner, in milliseconds. */
const WAIT = 150;

/** How long a spinner stays once it shows, in milliseconds, so it never flickers. */
const STAY = 400;

/** A spinner as it appears. */
const appear = style.keyframes({
    from: { opacity: 0 },
    to: { opacity: 1 },
});

/** The styles every button shares. */
const styles = style.create({
    button: {
        display: "inline-flex",
        flexShrink: 0,
        verticalAlign: "middle",
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
    busy: {
        position: "relative",
        cursor: { default: null, ":is([aria-busy=true])": "progress" },
        opacity: { default: null, ":is([aria-busy=true])": 1 },
    },
    label: {
        display: "inherit",
        alignItems: "inherit",
        justifyContent: "inherit",
        gap: "inherit",
        transitionProperty: "opacity",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    hidden: {
        opacity: 0,
    },
    spinner: {
        position: "absolute",
        inset: 0,
        display: "grid",
        placeItems: "center",
        animationName: appear,
        animationDuration: motion.durationShort,
        animationFillMode: "both",
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
    /** Whether its action runs: it refuses clicks, and shows a spinner in place of its label. */
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
    const isBusy = (): boolean => button.loading === true;
    const isWaiting = untrack(() => "loading" in properties);

    // show a spinner once its action takes a while, and for a while
    const { clock } = useEnvironment();
    const [isSpinning, setSpinning] = createSignal(false, { ownedWrite: true });
    let timer: number | undefined;
    let shownAt: number | undefined;
    if (isWaiting) {
        createEffect(isBusy, (isRunning) => {
            // wait until it takes a while, and until it stayed a while
            clock.clearTimeout(timer);
            const since = shownAt === undefined ? STAY : clock.now() - shownAt;
            const wait = isRunning ? WAIT : Math.max(STAY - since, 0);
            if (isRunning !== untrack(isSpinning)) {
                timer = clock.setTimeout(() => {
                    shownAt = isRunning ? clock.now() : undefined;
                    setSpinning(isRunning);
                }, wait);
            }
        });
        onCleanup(() => clock.clearTimeout(timer));
    }

    // refuse clicks while its action runs, the focus staying where it is
    const click = (event: MouseEvent & { readonly currentTarget: HTMLButtonElement }): void => {
        const told: unknown = button.onClick;
        if (isBusy()) {
            event.preventDefault();
            event.stopPropagation();
        } else if (typeof told === "function") {
            Reflect.apply(told, undefined, [event]);
        }
    };

    // mark the part and give it its styles with the caller's last
    const part: PartAttributes = merge(
        () =>
            style.attributes(
                [buttonVariants(button), isWaiting && styles.busy, join(), button.xstyle],
                button.style,
            ),
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
                return isBusy() ? "true" : undefined;
            },
            get disabled() {
                return button.disabled === true ? true : undefined;
            },
            get "aria-disabled"() {
                return isBusy() ? "true" : button["aria-disabled"];
            },
            get children() {
                return isWaiting ? (
                    <>
                        <span {...style.attrs(styles.label, isSpinning() && styles.hidden)}>
                            {button.children}
                        </span>
                        <Show when={isSpinning()}>
                            <span {...style.attrs(styles.spinner)}>
                                <Spinner aria-hidden="true" />
                            </span>
                        </Show>
                    </>
                ) : (
                    button.children
                );
            },
        },
    );

    return useRender({
        render: untrack(() => button.render),
        defaultTagName: "button",
        part: part,
        properties: merge(
            untrack(() => button.render) === undefined ? { type: "button" as const } : {},
            rest,
            isWaiting ? { onClick: click } : {},
        ),
    });
}
