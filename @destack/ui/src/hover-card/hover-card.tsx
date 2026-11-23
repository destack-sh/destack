import { type ElementPartProperties, renderPart, mergeProperties } from "../part/index.ts";
import * as style from "@destack/style";
import { color, radius, shadow, space, stroke, width } from "@destack/theme/tokens.stylex";
import {
    createContext,
    createEffect,
    type JSX,
    merge,
    omit,
    useContext,
    untrack,
} from "@destack/view";
import { Hover } from "../interaction/index.ts";
import { HoverPopover, type HoverPopoverProperties } from "../popover/index.ts";
import { type Align, Position, type Side } from "../position/index.ts";
import { TopLayer } from "../layer/index.ts";

/** The wait before a resting pointer opens a hover card, in milliseconds. */
const DELAY = 700;

/** The wait before a hover card the pointer left closes, in milliseconds. */
const CLOSE_DELAY = 300;

/** The side and alignment of a hover card that sets neither. */
const DEFAULTS: Required<Pick<HoverCardContentProperties, "side" | "align">> = {
    side: "bottom",
    align: "center",
};

/** The hover card of the nearest hover card root, null outside one. */
const HoverCardContext = createContext<HoverPopover | null>(null);

/** The styles of a hover card. */
const styles = style.create({
    content: {
        width: width.hoverCard,
        maxWidth: `calc(100vw - 2 * ${space[1]})`,
        maxHeight: `calc(100dvh - 2 * ${space[1]})`,
        overflowWrap: "anywhere",
        inset: "auto",
        padding: space[4],
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[4],
        backgroundColor: color.popover,
        color: color.foreground,
        boxShadow: shadow.overlay,
    },
});

/** The properties of a hover card root, its open state included. */
export interface HoverCardProperties extends HoverPopoverProperties {
    /** The wait before a resting pointer opens the card in milliseconds, 700 by default. */
    readonly delay?: number;
    /** The wait before the card closes once the pointer leaves in milliseconds, 300 by default. */
    readonly closeDelay?: number;
}

/** The properties of a hover card's trigger, the native link's attributes included. */
export type HoverCardTriggerProperties = ElementPartProperties &
    Omit<
        JSX.AnchorHTMLAttributes<HTMLAnchorElement>,
        "class" | "style" | "ref" | "onPointerEnter" | "onPointerLeave"
    >;

/** The properties of a hover card's content, the native element's attributes included. */
export interface HoverCardContentProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "ref" | "onToggle" | "onPointerEnter" | "onPointerLeave"
> {
    /** The side of the trigger it opens on, bottom by default. */
    readonly side?: Side;
    /** The edge of the trigger it lines up with, center by default. */
    readonly align?: Align;
    /** The gap from the trigger in pixels, 4 by default. */
    readonly sideOffset?: number;
    /** The StyleX styles applied after the hover card's styles. */
    readonly xstyle?: style.Styles;
}

/** Read the hover card of the nearest hover card root, refusing elements outside one. */
export function useHoverCard(): HoverPopover {
    const card = useContext(HoverCardContext);
    if (card === null) {
        throw new TypeError("hover card elements need a hover card root around them");
    }

    return card;
}

/** Connect a link to the preview card that opens while a pointer rests on it. */
export function HoverCard(properties: HoverCardProperties): JSX.Element {
    const card = new HoverPopover(
        properties,
        () => properties.delay ?? DELAY,
        () => properties.closeDelay ?? CLOSE_DELAY,
    );

    return <HoverCardContext value={card}>{properties.children}</HoverCardContext>;
}

/** Render the element that opens its hover card on hover or focus after the delay. */
export function HoverCardTrigger(properties: HoverCardTriggerProperties): JSX.Element {
    const card = useHoverCard();
    const hover = new Hover({
        onHoverStart: () => card.openLater(),
        onHoverEnd: (event) => card.leave(event),
    });

    return renderPart(
        "a",
        "hover-card-trigger",
        properties,
        [],
        mergeProperties(hover.attributes(), {
            get "data-popup-open"() {
                return card.isOpen() ? "" : undefined;
            },
            ref: (element: HTMLElement) => card.setTrigger(element),
            onFocus: () => card.openLater(),
            onBlur: () => card.closeLater(),
        }),
    );
}

/** Render a preview in the top layer beside its link and open while the pointer rests on it. */
export function HoverCardContent(properties: HoverCardContentProperties): JSX.Element {
    // read the hover card and its placement
    const card = useHoverCard();
    const content = merge(DEFAULTS, properties);
    const rest = omit(content, "side", "align", "sideOffset", "xstyle", "style");

    // show and hide the card as it opens and closes
    createEffect(card.isOpen, (isOpen) => untrack(() => card.sync(isOpen)));

    // leave the card open while a pointer rests on it
    const hover = new Hover({
        onHoverStart: () => card.open(),
        onHoverEnd: () => card.closeLater(),
    });

    return (
        <TopLayer>
            <div
                id={card.id}
                popover="hint"
                data-slot="hover-card-content"
                data-open={card.isOpen() ? "" : undefined}
                data-closed={card.isOpen() ? undefined : ""}
                data-side={content.side}
                data-align={content.align}
                {...rest}
                ref={(element) => card.setContent(element)}
                onToggle={(event) => card.follow(event)}
                {...hover.attributes()}
                {...style.attributes(
                    [
                        styles.content,
                        Position.beside(content.side, content.align),
                        Position.offset(content.side, content.sideOffset),
                        content.xstyle,
                    ],
                    content.style,
                )}
            />
        </TopLayer>
    );
}
