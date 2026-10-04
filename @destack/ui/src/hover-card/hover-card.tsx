import * as style from "@destack/style";
import { color, radius, shadow, space, stroke } from "@destack/theme/tokens.stylex";
import type { JSX } from "@solidjs/web";
import { createContext, merge, omit, useContext } from "solid-js";
import {
    HoverPopover,
    placementStyle,
    type PopoverAlign,
    type PopoverSide,
} from "../popover/index.ts";
import { TopLayer } from "../layer/index.ts";

/** The wait before a resting pointer opens a hover card, Radix's default in milliseconds. */
const OPEN_DELAY = 700;

/** The wait before a hover card the pointer left closes, Radix's default in milliseconds. */
const CLOSE_DELAY = 300;

/** The width of a hover card, Tailwind's w-64 that shadcn/ui's hover card takes. */
const HOVER_CARD_WIDTH = "16rem";

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
        boxSizing: "border-box",
        width: HOVER_CARD_WIDTH,
        inset: { default: 0, "@supports (position-area: block-end)": "auto" },
        margin: { default: "auto", "@supports (position-area: block-end)": space[1] },
        padding: space[4],
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[3],
        backgroundColor: color.popover,
        color: color.popoverForeground,
        boxShadow: shadow.overlay,
    },
});

/** The properties of a hover card root. */
export interface HoverCardProperties {
    /** The wait before a resting pointer opens the card in milliseconds, 700 by default. */
    readonly openDelay?: number;
    /** The wait before the card closes once the pointer leaves in milliseconds, 300 by default. */
    readonly closeDelay?: number;
    /** The trigger and content. */
    readonly children?: JSX.Element;
}

/** The properties of a hover card's trigger, the native link's attributes included. */
export type HoverCardTriggerProperties = Omit<
    JSX.AnchorHTMLAttributes<HTMLAnchorElement>,
    "ref" | "onPointerEnter" | "onPointerLeave" | "onFocus" | "onBlur"
>;

/** The properties of a hover card's content, the native element's attributes included. */
export interface HoverCardContentProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "style" | "ref" | "onToggle" | "onPointerEnter" | "onPointerLeave"
> {
    /** The side of the trigger it opens on, bottom by default. */
    readonly side?: PopoverSide;
    /** The edge of the trigger it lines up with, center by default. */
    readonly align?: PopoverAlign;
    /** The StyleX styles applied after the hover card's styles. */
    readonly style?: style.Styles;
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
        () => properties.openDelay ?? OPEN_DELAY,
        () => properties.closeDelay ?? CLOSE_DELAY,
    );

    return <HoverCardContext value={card}>{properties.children}</HoverCardContext>;
}

/** Render a link that opens its hover card on hover or focus after the delay. */
export function HoverCardTrigger(properties: HoverCardTriggerProperties): JSX.Element {
    const card = useHoverCard();

    return (
        <a
            data-slot="hover-card-trigger"
            {...properties}
            ref={(element) => card.setTrigger(element)}
            onPointerEnter={() => card.openLater()}
            onPointerLeave={() => card.closeLater()}
            onFocus={() => card.openLater()}
            onBlur={() => card.closeLater()}
        />
    );
}

/** Render a preview in the top layer beside its link, kept open while the pointer rests on it. */
export function HoverCardContent(properties: HoverCardContentProperties): JSX.Element {
    // read the hover card and its placement
    const card = useHoverCard();
    const content = merge(DEFAULTS, properties);
    const rest = omit(content, "side", "align", "style");

    return (
        <TopLayer>
            <div
                id={card.id}
                popover="hint"
                data-slot="hover-card-content"
                data-side={content.side}
                {...rest}
                ref={(element) => card.setContent(element)}
                onToggle={(event) => card.follow(event)}
                onPointerEnter={() => card.open()}
                onPointerLeave={() => card.closeLater()}
                {...style.attrs(
                    styles.content,
                    placementStyle(content.side, content.align),
                    content.style,
                )}
            />
        </TopLayer>
    );
}
