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
import {
    type ComponentProperties,
    type DeclaredProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";
import { field } from "@destack/object";
import { t } from "@destack/locale";
import { mergeProperties, type PartAttributes, type Render, renderPart } from "../part/index.ts";
import { Hint } from "../tooltip/index.ts";
import { useJoin } from "../join/index.ts";
import { Spinner } from "../spinner/index.ts";

/** The sizes that set their text in the extra small style. */
const SMALL_SIZES: ReadonlySet<ButtonSize> = new Set(["sm"]);

/** The variants that are clear at rest and fill under the pointer. */
const HOVER_VARIANTS: ReadonlySet<ButtonVariant> = new Set(["outline", "ghost"]);

/** The looks of a button, from most to least prominent, each with the case it suits. */
const VARIANTS = {
    default: {
        title: t.context("option")`Default`,
        description: t`The one main action of a place.`,
    },
    destructive: {
        title: t.context("option")`Destructive`,
        description: t`An action that deletes or cannot be undone.`,
    },
    outline: {
        title: t.context("option")`Outline`,
        description: t`A second action beside the main one.`,
    },
    secondary: {
        title: t.context("option")`Secondary`,
        description: t`A quieter action on a busy surface.`,
    },
    ghost: {
        title: t.context("option")`Ghost`,
        description: t`An action in a toolbar or a row.`,
    },
    link: {
        title: t.context("option")`Link`,
        description: t`An action that reads as text in its line.`,
    },
};

/** The heights and paddings of a button. */
const SIZES = {
    sm: { title: t.context("option")`Small` },
    default: { title: t.context("option")`Default` },
    lg: { title: t.context("option")`Large` },
};

/** The corners of a button. */
const SHAPES = {
    default: { title: t.context("option")`Default` },
    round: { title: t.context("option")`Round` },
};

/** The properties people change in a button's controls and inspector. */
const BUTTON_PROPERTIES = {
    variant: field
        .enum(VARIANTS)
        .labeled(t.context("label")`Variant`)
        .described(t`How much the button stands out among its neighbours.`)
        .default("default"),
    size: field
        .enum(SIZES)
        .labeled(t.context("label")`Size`)
        .described(t`How tall the button is, and how much room its label gets.`)
        .default("default"),
    shape: field
        .enum(SHAPES)
        .labeled(t.context("label")`Shape`)
        .described(t`Round suits a lone icon, such as a chat's send button.`)
        .default("default"),
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(t`Refuses presses where the action cannot run now.`)
        .default(false),
    loading: field
        .boolean()
        .labeled(t.context("label")`Loading`)
        .described(t`Shows a spinner in place of the label while its action runs.`)
        .optional(),
    isIconOnly: field
        .boolean()
        .labeled(t.context("label")`Icon only`)
        .described(t`Shows only the icon, square, named by its label for assistive technology.`)
        .default(false),
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
    round: {
        borderRadius: radius.full,
    },
    // set a link in the line it sits in, as text
    flush: {
        height: "auto",
        paddingInline: 0,
        verticalAlign: "baseline",
    },
    busy: {
        position: "relative",
        cursor: { default: null, ":is([aria-busy=true])": "progress" },
        // dim it while disabled as every control does, but not while its action runs
        opacity: {
            default: null,
            ":is(:disabled, [aria-disabled=true])": 0.5,
            ":is([aria-busy=true])": 1,
        },
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
        animationTimingFunction: motion.easingStandard,
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
    sm: { height: size[1], gap: space[1], paddingInline: space[2] },
    default: { height: size[2], paddingInline: space[3] },
    lg: { height: size[3], paddingInline: space[4] },
});

/** The square of each size, for a button showing only its icon. */
const squares = style.create({
    sm: { width: size[1], height: size[1], paddingInline: 0 },
    default: { width: size[2], height: size[2], paddingInline: 0 },
    lg: { width: size[3], height: size[3], paddingInline: 0 },
});

/** The look of a button, from most to least prominent. */
export type ButtonVariant = keyof typeof VARIANTS;

/** The height and padding of a button. */
export type ButtonSize = keyof typeof SIZES;

/** The corners of a button. */
export type ButtonShape = keyof typeof SHAPES;

/** The properties of a button besides its declared ones: its native attributes and its part. */
export interface ButtonAttributes extends Omit<
    JSX.ButtonHTMLAttributes<HTMLButtonElement>,
    "class" | keyof typeof BUTTON_PROPERTIES
> {
    /** The part's name for styling, `button` by default. */
    readonly "data-slot"?: string;
    /** The StyleX styles applied after the button's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with the button's attributes, such as a link. */
    readonly render?: Render;
    /** What its tooltip says, its label where it shows only its icon, and no tooltip otherwise. */
    readonly tooltip?: JSX.Element;
}

/** The look of a button: its variant, size, shape and whether it shows only its icon. */
type ButtonLook = Pick<
    DeclaredProperties<typeof BUTTON_PROPERTIES>,
    "variant" | "size" | "shape" | "isIconOnly"
>;

/** Return the StyleX styles of a button in its look. */
function buttonVariants(look: ButtonLook): style.Styles {
    return [
        SMALL_SIZES.has(look.size) ? text.xs : text.sm,
        state.focus,
        state.disabled,
        state.press,
        styles.button,
        HOVER_VARIANTS.has(look.variant) && state.hover,
        variants[look.variant],
        sizes[look.size],
        look.variant === "link" && styles.flush,
        look.shape === "round" && styles.round,
        look.isIconOnly && squares[look.size],
    ];
}

/** Render a native button in a variant and size, its styles at hand for other elements. */
export const Button = Object.assign(
    defineComponent({
        name: "button",
        properties: BUTTON_PROPERTIES,
        render(
            properties: RenderProperties<ButtonAttributes, typeof BUTTON_PROPERTIES>,
        ): JSX.Element {
            // join the button to its neighbours in a button group
            const join = useJoin();
            const rest = omit(
                properties,
                "variant",
                "size",
                "shape",
                "isIconOnly",
                "loading",
                "disabled",
                "children",
                "tooltip",
            );
            const isBusy = (): boolean => properties.loading === true;
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
            const click = (
                event: MouseEvent & { readonly currentTarget: HTMLButtonElement },
            ): void => {
                const told: unknown = properties.onClick;
                if (isBusy()) {
                    event.preventDefault();
                    event.stopPropagation();
                } else if (typeof told === "function") {
                    Reflect.apply(told, undefined, [event]);
                }
            };

            // mark the part and give it its styles with the caller's last, a tooltip's attributes first
            const button = (attributes: PartAttributes): JSX.Element =>
                renderPart(
                    "button",
                    "button",
                    merge(
                        untrack(() => properties.render) === undefined
                            ? { type: "button" as const }
                            : {},
                        mergeProperties(attributes, rest),
                        isWaiting ? { onClick: click } : {},
                    ),
                    () => [buttonVariants(properties), isWaiting && styles.busy, join()],
                    {
                        get "data-variant"() {
                            return properties.variant;
                        },
                        get "data-size"() {
                            return properties.size;
                        },
                        get "aria-busy"() {
                            return isBusy() ? "true" : undefined;
                        },
                        get disabled() {
                            return properties.disabled ? true : undefined;
                        },
                        get "aria-disabled"() {
                            return isBusy() ? "true" : properties["aria-disabled"];
                        },
                        get children() {
                            return isWaiting ? (
                                <>
                                    <span
                                        {...style.attrs(
                                            styles.label,
                                            isSpinning() && styles.hidden,
                                        )}
                                    >
                                        {properties.children}
                                    </span>
                                    <Show when={isSpinning()}>
                                        <span {...style.attrs(styles.spinner)}>
                                            <Spinner aria-hidden="true" />
                                        </span>
                                    </Show>
                                </>
                            ) : (
                                properties.children
                            );
                        },
                    },
                );

            // name an icon by its label in a hint, or hint what the caller gives
            return (
                <Hint
                    text={
                        properties.tooltip ??
                        (properties.isIconOnly ? properties["aria-label"] : undefined)
                    }
                >
                    {button}
                </Hint>
            );
        },
    }),
    {
        /** Read the styles of a button, for an element that only looks like one. */
        styles: (look: Partial<ButtonLook>): style.Styles =>
            buttonVariants({ ...Button.defaults, ...look }),
    },
);

/** The properties of a button, the native button's attributes included. */
export type ButtonProperties = ComponentProperties<typeof Button>;
