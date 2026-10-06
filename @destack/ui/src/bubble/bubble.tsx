import * as style from "@destack/style";
import { color, radius, space, stroke } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { createContext, type JSX, merge, omit, useContext } from "@destack/view";

/** The variant and alignment of a bubble that sets neither. */
const DEFAULTS: Required<Pick<BubbleProperties, "variant" | "align">> = {
    variant: "default",
    align: "start",
};

/** The side and corner of a bubble's reactions that sets neither. */
const REACTION_DEFAULTS: Required<Pick<BubbleReactionsProperties, "side" | "align">> = {
    side: "bottom",
    align: "end",
};

/** The styles of a bubble and its elements. */
const styles = style.create({
    group: {
        display: "flex",
        flexDirection: "column",
        gap: space[2],
        minWidth: 0,
    },
    bubble: {
        position: "relative",
        display: "flex",
        flexDirection: "column",
        gap: space[1],
        width: "fit-content",
        maxWidth: "80%",
        minWidth: 0,
    },
    end: {
        alignSelf: "flex-end",
    },
    ghost: {
        maxWidth: "100%",
    },
    content: {
        overflow: "hidden",
        width: "fit-content",
        maxWidth: "100%",
        minWidth: 0,
        paddingInline: space[3],
        paddingBlock: space[2],
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: "transparent",
        borderRadius: radius[5],
        overflowWrap: "anywhere",
    },
    reactions: {
        position: "absolute",
        zIndex: 1,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        gap: space[1],
        width: "fit-content",
        paddingInline: space[2],
        paddingBlock: space[1],
        borderRadius: radius.full,
        backgroundColor: color.muted,
        boxShadow: `0 0 0 ${stroke.ring} ${color.card}`,
    },
    top: { top: 0, transform: "translateY(-75%)" },
    bottom: { bottom: 0, transform: "translateY(75%)" },
    reactionsStart: { insetInlineStart: space[3] },
    reactionsEnd: { insetInlineEnd: space[3] },
});

/** The colors of a bubble's content in each variant. */
const variants = style.create({
    default: { backgroundColor: color.primary, color: color.primaryForeground },
    secondary: { backgroundColor: color.secondary, color: color.secondaryForeground },
    muted: { backgroundColor: color.muted, color: color.foreground },
    tinted: {
        backgroundColor: `color-mix(in oklab, ${color.primary} 15%, ${color.background})`,
        color: color.foreground,
    },
    outline: {
        borderColor: color.border,
        backgroundColor: color.background,
        color: color.foreground,
    },
    ghost: {
        padding: 0,
        borderWidth: 0,
        borderRadius: 0,
        backgroundColor: "transparent",
        color: color.foreground,
    },
    destructive: {
        backgroundColor: `color-mix(in oklab, ${color.destructive} 10%, transparent)`,
        color: color.destructive,
    },
});

/** The variant of the nearest bubble, which its content takes its colors from. */
const BubbleContext = createContext<() => BubbleVariant>(() => "default");

/** The look of a bubble, from most to least prominent. */
export type BubbleVariant =
    | "default"
    | "secondary"
    | "muted"
    | "tinted"
    | "outline"
    | "ghost"
    | "destructive";

/** The side a bubble or its reactions sit on. */
export type BubbleAlign = "start" | "end";

/** The edge of a bubble its reactions overlap. */
export type BubbleReactionsSide = "top" | "bottom";

/** The properties of an element of a bubble, the native element's attributes included. */
export type BubbleElementProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
};

/** The properties of a bubble, the native element's attributes included. */
export interface BubbleProperties extends BubbleElementProperties {
    /** The look, default by default. */
    readonly variant?: BubbleVariant;
    /** The side, start by default. */
    readonly align?: BubbleAlign;
}

/** The properties of a bubble's reactions, the native element's attributes included. */
export interface BubbleReactionsProperties extends BubbleElementProperties {
    /** The edge they overlap, bottom by default. */
    readonly side?: BubbleReactionsSide;
    /** The corner they sit in, end by default. */
    readonly align?: BubbleAlign;
}

/** Render consecutive bubbles stacked together. */
export function BubbleGroup(properties: BubbleElementProperties): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            data-slot="bubble-group"
            {...rest}
            {...style.attributes([styles.group, properties.xstyle], properties.style)}
        />
    );
}

/** Render the text of a message in a rounded bubble, with its reactions on its edge. */
export function Bubble(properties: BubbleProperties): JSX.Element {
    const bubble = merge(DEFAULTS, properties);
    const rest = omit(bubble, "variant", "align", "xstyle", "style");

    return (
        <BubbleContext value={() => bubble.variant}>
            <div
                data-slot="bubble"
                data-variant={bubble.variant}
                data-align={bubble.align}
                {...rest}
                {...style.attributes(
                    [
                        styles.bubble,
                        bubble.align === "end" && styles.end,
                        bubble.variant === "ghost" && styles.ghost,
                        bubble.xstyle,
                    ],
                    bubble.style,
                )}
            />
        </BubbleContext>
    );
}

/** Render a bubble's text in the bubble's colors. */
export function BubbleContent(properties: BubbleElementProperties): JSX.Element {
    const variant = useContext(BubbleContext);
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            data-slot="bubble-content"
            {...rest}
            {...style.attributes(
                [text.callout, styles.content, variants[variant()], properties.xstyle],
                properties.style,
            )}
        />
    );
}

/** Render the reactions to a bubble on its edge. */
export function BubbleReactions(properties: BubbleReactionsProperties): JSX.Element {
    const reactions = merge(REACTION_DEFAULTS, properties);
    const rest = omit(reactions, "side", "align", "xstyle", "style");

    return (
        <div
            data-slot="bubble-reactions"
            data-side={reactions.side}
            data-align={reactions.align}
            {...rest}
            {...style.attributes(
                [
                    text.footnote,
                    styles.reactions,
                    styles[reactions.side],
                    reactions.align === "start" ? styles.reactionsStart : styles.reactionsEnd,
                    reactions.xstyle,
                ],
                reactions.style,
            )}
        />
    );
}
