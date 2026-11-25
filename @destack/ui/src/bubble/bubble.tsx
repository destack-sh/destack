import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";
import { state } from "@destack/theme/state";
import { text } from "@destack/theme/text";
import { color, radius, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import { createContext, type JSX, merge, omit, useContext } from "@destack/view";
import { useMessageAlign } from "../message/index.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import { bubbleMarker } from "./marker.stylex.ts";

/** The variant of a bubble that sets none. */
const DEFAULTS = { variant: "default" } as const;

/** The place of a bubble's reactions that set none. */
const REACTION_DEFAULTS = { side: "bottom", align: "end" } as const;

/** The widest a bubble grows, so the sides of a conversation stay apart. */
const WIDEST = "80%";

/** The styles of a bubble and its parts. */
const styles = style.create({
    bubble: {
        position: "relative",
        display: "flex",
        flexDirection: "column",
        width: "fit-content",
        maxWidth: WIDEST,
        minWidth: 0,
    },
    end: {
        alignSelf: "flex-end",
        marginInlineStart: "auto",
    },
    wide: {
        width: "100%",
        maxWidth: "100%",
    },
    content: {
        minWidth: 0,
        paddingBlock: space[2],
        paddingInline: space[3],
        borderWidth: stroke.border,
        borderColor: "transparent",
        borderRadius: radius[6],
        textAlign: "start",
        overflowWrap: "anywhere",
        whiteSpace: "pre-wrap",
    },
    // lighten or darken it under a mouse or pen, whatever its color is
    interactive: {
        cursor: "pointer",
        backgroundImage: {
            default: null,
            ":hover": {
                default: null,
                [media.hover]: `linear-gradient(${color.hover}, ${color.hover})`,
            },
        },
    },
    // tighten the corners beside its neighbours in a group, on the side it stands at
    startCorners: {
        borderStartStartRadius: {
            default: null,
            [style.when.ancestor(":not(:first-child)", bubbleMarker)]: radius[2],
        },
        borderEndStartRadius: {
            default: null,
            [style.when.ancestor(":not(:last-child)", bubbleMarker)]: radius[2],
        },
    },
    endCorners: {
        borderStartEndRadius: {
            default: null,
            [style.when.ancestor(":not(:first-child)", bubbleMarker)]: radius[2],
        },
        borderEndEndRadius: {
            default: null,
            [style.when.ancestor(":not(:last-child)", bubbleMarker)]: radius[2],
        },
    },
    reactions: {
        position: "absolute",
        zIndex: 1,
        display: "flex",
        alignItems: "center",
        gap: space[1],
        paddingInline: { default: space[2], ":has(button)": 0 },
        borderWidth: stroke.border,
        borderColor: color.background,
        borderRadius: radius.full,
        backgroundColor: color.muted,
        whiteSpace: "nowrap",
    },
    group: {
        display: "flex",
        flexDirection: "column",
        gap: `calc(${space[1]} / 2)`,
        minWidth: 0,
        width: "100%",
    },
});

/** The colors of each variant. */
const variants = style.create({
    default: {
        backgroundColor: swatch.accentSolid,
        color: swatch.accentLabel,
    },
    secondary: {
        backgroundColor: color.muted,
        color: color.foreground,
    },
    muted: {
        backgroundColor: `color-mix(in oklab, ${color.muted} 50%, transparent)`,
        color: color.mutedForeground,
    },
    tinted: {
        backgroundColor: swatch.accentTint,
        color: swatch.accentTintText,
    },
    outline: {
        borderColor: color.border,
        backgroundColor: color.background,
        color: color.foreground,
    },
    ghost: {
        paddingInline: 0,
        paddingBlock: 0,
        borderRadius: 0,
        color: color.foreground,
        borderColor: { default: "transparent", [media.forcedColors]: "Canvas" },
    },
    destructive: {
        backgroundColor: swatch.destructiveTint,
        color: swatch.destructiveTintText,
    },
});

/** The places of a bubble's reactions, half over its edge. */
const sides = style.create({
    top: { insetBlockEnd: `calc(100% - ${space[2]})` },
    bottom: { insetBlockStart: `calc(100% - ${space[2]})` },
    start: { insetInlineStart: space[3] },
    end: { insetInlineEnd: space[3] },
});

/** The look of a bubble. */
export type BubbleVariant = keyof typeof variants;

/** The side of a conversation a bubble stands at. */
export type BubbleAlign = "start" | "end";

/** The properties of an element of a bubble, the native element's attributes included. */
export type BubbleElementProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties;

/** The properties of a bubble. */
export type BubbleProperties = BubbleElementProperties & {
    /** The look, the accent's by default. */
    readonly variant?: BubbleVariant;
    /** The side it stands at, its message's by default. */
    readonly align?: BubbleAlign;
};

/** Whether a bubble stands in a group. */
const BubbleGroupContext = createContext(false);

/** The look and side of the nearest bubble, the defaults outside one. */
const BubbleContext = createContext<{
    readonly variant: BubbleVariant;
    readonly align: BubbleAlign;
}>({ ...DEFAULTS, align: "start" });

/** Render the surface of a message, as wide as its content up to most of the row. */
export function Bubble(properties: BubbleProperties): JSX.Element {
    // stand at the side of the message around it
    const messageAlign = useMessageAlign();
    const bubble = {
        get variant() {
            return properties.variant ?? DEFAULTS.variant;
        },
        get align() {
            return properties.align ?? messageAlign();
        },
    };
    const rest = omit(properties, "variant", "align");

    return (
        <BubbleContext value={bubble}>
            {renderPart(
                "div",
                "bubble",
                rest,
                () => [
                    bubbleMarker,
                    styles.bubble,
                    bubble.variant === "ghost" && styles.wide,
                    bubble.align === "end" && styles.end,
                ],
                {
                    get "data-variant"() {
                        return bubble.variant;
                    },
                    get "data-align"() {
                        return bubble.align;
                    },
                },
            )}
        </BubbleContext>
    );
}

/** Render the content of the nearest bubble, a link or a button where it renders one. */
export function BubbleContent(properties: BubbleElementProperties): JSX.Element {
    const bubble = useContext(BubbleContext);
    const isGrouped = useContext(BubbleGroupContext);

    return renderPart("div", "bubble-content", properties, () => [
        text.sm,
        styles.content,
        isGrouped && (bubble.align === "end" ? styles.endCorners : styles.startCorners),
        variants[bubble.variant],
        properties.render !== undefined && [state.focus, styles.interactive],
    ]);
}

/** The properties of a bubble's reactions. */
export type BubbleOverlayProperties = BubbleElementProperties & {
    /** The edge they stand over, the bottom by default. */
    readonly side?: "top" | "bottom";
    /** The end of the edge they stand at, the end by default. */
    readonly align?: BubbleAlign;
};

/** Render the reactions over the edge of the nearest bubble. */
export function BubbleOverlay(properties: BubbleOverlayProperties): JSX.Element {
    const reactions = merge(REACTION_DEFAULTS, properties);
    const rest = omit(properties, "side", "align");

    return renderPart(
        "div",
        "bubble-overlay",
        rest,
        () => [text.xs, styles.reactions, sides[reactions.side], sides[reactions.align]],
        {
            get "data-side"() {
                return reactions.side;
            },
            get "data-align"() {
                return reactions.align;
            },
        },
    );
}

/** Render the bubbles of one person in a row, their corners tight where they meet. */
export function BubbleGroup(properties: BubbleElementProperties): JSX.Element {
    return (
        <BubbleGroupContext value={true}>
            {renderPart("div", "bubble-group", properties, styles.group)}
        </BubbleGroupContext>
    );
}
