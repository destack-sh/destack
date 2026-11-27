import { swatches } from "@destack/theme/swatch";
import { Hue } from "@destack/style";
import { t } from "@destack/locale";
import { field } from "@destack/object";
import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";
import { state } from "@destack/theme/state";
import { text } from "@destack/theme/text";
import { color, radius, space, stroke } from "@destack/theme/tokens.stylex";
import { createContext, type JSX, omit, useContext } from "@destack/view";
import { MESSAGE_ALIGNS, type MessageAlign, useMessageAlign } from "../message/index.ts";
import { messageMarker } from "../message/marker.stylex.ts";
import { Fill } from "../fill/index.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import { bubbleMarker } from "./marker.stylex.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The widest a bubble grows, so the sides of a conversation stay apart. */
const WIDEST = "80%";

/** The widest a bubble grows in a message, measured against the message. */
const WIDEST_IN_MESSAGE = "80cqi";

/** The styles of a bubble and its parts. */
const styles = style.create({
    bubble: {
        position: "relative",
        display: "flex",
        flexDirection: "column",
        width: "fit-content",
        maxWidth: {
            default: WIDEST,
            [style.when.ancestor("[data-slot=message]", messageMarker)]: WIDEST_IN_MESSAGE,
        },
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
    ghost: {
        paddingInline: 0,
        paddingBlock: 0,
        borderRadius: 0,
        color: color.foreground,
        borderColor: { default: "transparent", [media.forcedColors]: "Canvas" },
    },
    ink: (foreground: string) => ({
        color: foreground,
    }),
    group: {
        display: "flex",
        flexDirection: "column",
        gap: `calc(${space[1]} / 2)`,
        minWidth: 0,
        width: "100%",
    },
});

/** The places of a bubble's reactions, half over its edge. */
const sides = style.create({
    top: { insetBlockEnd: `calc(100% - ${space[2]})` },
    bottom: { insetBlockStart: `calc(100% - ${space[2]})` },
    start: { insetInlineStart: space[3] },
    end: { insetInlineEnd: space[3] },
});

/** The look of a bubble: a fill, or its text alone. */
export type BubbleVariant = Fill | "ghost";

/** The properties of an element of a bubble, the native element's attributes included. */
export type BubbleElementProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties;

/** The properties people change in a bubble's controls and inspector. */
const BUBBLE_PROPERTIES = {
    variant: field
        .enum({
            ...Fill.choices,
            ghost: {
                title: t.context("option")`Ghost`,
                description: t`Text alone in its color, as for a long answer.`,
            },
        })
        .labeled(t.context("label")`Variant`)
        .described(t`How the bubble is filled in its color.`)
        .default("solid"),
    color: field
        .enum(Hue.choices)
        .labeled(t.context("label")`Color`)
        .described(t`The hue it is drawn in, the accent when absent.`)
        .optional(),
    align: field
        .enum(MESSAGE_ALIGNS)
        .labeled(t.context("label")`Align`)
        .described(t`The side of the conversation it stands at.`)
        .optional(),
};

/** Whether a bubble stands in a group. */
const BubbleGroupContext = createContext(false);

/** The look and side of the nearest bubble, the defaults outside one. */
const BubbleContext = createContext<{
    readonly variant: BubbleVariant;
    readonly color: Hue | undefined;
    readonly align: MessageAlign;
}>({ variant: "solid", color: undefined, align: "start" });

/** Render the surface of a message, as wide as its content up to most of the row. */
export const Bubble = defineComponent({
    name: "bubble",
    properties: BUBBLE_PROPERTIES,
    render(
        properties: RenderProperties<BubbleElementProperties, typeof BUBBLE_PROPERTIES>,
    ): JSX.Element {
        // stand at the side of the message around it
        const messageAlign = useMessageAlign();
        const bubble = {
            get variant() {
                return properties.variant;
            },
            get color() {
                return properties.color;
            },
            get align() {
                return properties.align ?? messageAlign();
            },
        };
        const rest = omit(properties, "variant", "color", "align");

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
                        get "data-color"() {
                            return bubble.color;
                        },
                        get "data-align"() {
                            return bubble.align;
                        },
                    },
                )}
            </BubbleContext>
        );
    },
});

/** The properties of a bubble, its native attributes included. */
export type BubbleProperties = ComponentProperties<typeof Bubble>;

/** Render the content of the nearest bubble, a link or a button where it renders one. */
export function BubbleContent(properties: BubbleElementProperties): JSX.Element {
    const bubble = useContext(BubbleContext);
    const isGrouped = useContext(BubbleGroupContext);

    return renderPart("div", "bubble-content", properties, () => [
        text.sm,
        styles.content,
        isGrouped && (bubble.align === "end" ? styles.endCorners : styles.startCorners),
        bubble.variant === "ghost"
            ? [styles.ghost, bubble.color !== undefined && styles.ink(swatches[bubble.color].text)]
            : Fill.styles(bubble.variant, bubble.color),
        properties.render !== undefined && [state.focus, styles.interactive],
    ]);
}

/** The properties people change in a bubble overlay's controls and inspector. */
const BUBBLE_OVERLAY_PROPERTIES = {
    side: field
        .enum({
            top: { title: t.context("option")`Top` },
            bottom: { title: t.context("option")`Bottom` },
        })
        .labeled(t.context("label")`Side`)
        .described(t`The edge they stand over.`)
        .default("bottom"),
    align: field
        .enum(MESSAGE_ALIGNS)
        .labeled(t.context("label")`Align`)
        .described(t`The end of the edge they stand at.`)
        .default("end"),
};

/** Render the reactions over the edge of the nearest bubble. */
export const BubbleOverlay = defineComponent({
    name: "bubbleOverlay",
    properties: BUBBLE_OVERLAY_PROPERTIES,
    render(
        properties: RenderProperties<BubbleElementProperties, typeof BUBBLE_OVERLAY_PROPERTIES>,
    ): JSX.Element {
        const rest = omit(properties, "side", "align");

        return renderPart(
            "div",
            "bubble-overlay",
            rest,
            () => [text.xs, styles.reactions, sides[properties.side], sides[properties.align]],
            {
                get "data-side"() {
                    return properties.side;
                },
                get "data-align"() {
                    return properties.align;
                },
            },
        );
    },
});

/** The properties of a bubble's reactions, their native attributes included. */
export type BubbleOverlayProperties = ComponentProperties<typeof BubbleOverlay>;

/** Render the bubbles of one person in a row, their corners tight where they meet. */
export function BubbleGroup(properties: BubbleElementProperties): JSX.Element {
    return (
        <BubbleGroupContext value={true}>
            {renderPart("div", "bubble-group", properties, styles.group)}
        </BubbleGroupContext>
    );
}
