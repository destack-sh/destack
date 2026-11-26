import {
    type ElementPartProperties,
    renderPart,
    mergeProperties,
    type Render,
} from "../part/index.ts";
import * as style from "@destack/style";
import { color, radius, shadow, space, stroke, width } from "@destack/theme/tokens.stylex";
import {
    createContext,
    createEffect,
    type JSX,
    omit,
    onCleanup,
    useContext,
    untrack,
} from "@destack/view";
import { Hover } from "../interaction/index.ts";
import { HoverPopover } from "../popover/index.ts";
import { type Anchor, Position, PositionAnchor, ANCHOR_PROPERTIES } from "../position/index.ts";
import {
    controlOf,
    OpenChange,
    type OpenChildren,
    type OpenProperties,
    type OpenReason,
    TopLayer,
    type TriggerProperties,
} from "../layer/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The wait before a resting pointer opens a hover card, in milliseconds. */
const DELAY = 700;

/** The wait before a hover card the pointer left closes, in milliseconds. */
const CLOSE_DELAY = 300;

/** The properties people change in a hover card's controls and inspector. */
const HOVER_CARD_CONTENT_PROPERTIES = {
    side: ANCHOR_PROPERTIES.side.default("bottom"),
    align: ANCHOR_PROPERTIES.align.default("center"),
};

/** The hover card of the nearest hover card root, null outside one. */
const HoverCardContext = createContext<HoverCardControl | null>(null);

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

/** How a hover card opens. */
export interface HoverCardOptions extends OpenProperties {
    /** The wait before a resting pointer opens the card in milliseconds, 700 by default. */
    readonly delay?: number;
    /** The wait before the card closes once the pointer leaves in milliseconds, 300 by default. */
    readonly closeDelay?: number;
}

/** A hover card: a popover a resting pointer opens, for one trigger or many. */
export class HoverCardControl<Payload = unknown> extends HoverPopover<Payload> {
    /** Create the state of a hover card, open when its options ask for it. */
    constructor(options: HoverCardOptions = {}) {
        super(
            options,
            () => options.delay ?? DELAY,
            () => options.closeDelay ?? CLOSE_DELAY,
        );
    }
}

/** The properties of a hover card root, its open state included. */
export interface HoverCardProperties<Payload = unknown> extends HoverCardOptions {
    /** The card that triggers elsewhere open, which holds the open state in the root's place. */
    readonly control?: HoverCardControl<Payload>;
    /** The trigger and content, or what it shows for the trigger it opened from. */
    readonly children?: OpenChildren<Payload>;
}

/** The properties of a hover card's trigger, the native link's attributes included. */
export type HoverCardTriggerProperties<Payload = unknown> = ElementPartProperties &
    Omit<JSX.AnchorHTMLAttributes<HTMLAnchorElement>, "class" | "style"> &
    TriggerProperties<Payload, HoverCardControl<Payload>>;

/** The properties of a hover card's content besides its declared ones: its anchor and gap. */
export interface HoverCardContentAttributes extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class"
> {
    /** What it is placed beside in place of its trigger: an element, or a box in the viewport. */
    readonly anchor?: Anchor | undefined;
    /** The gap from the trigger in pixels, 4 by default. */
    readonly sideOffset?: number;
    /** The StyleX styles applied after the hover card's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
}

/** Read the hover card of the nearest hover card root, refusing elements outside one. */
export function useHoverCard(): HoverCardControl {
    const card = useContext(HoverCardContext);
    if (card === null) {
        throw new TypeError("hover card elements need a hover card root around them");
    }

    return card;
}

/** Connect a link to the preview card that opens while a pointer rests on it. */
export function HoverCard<Payload = unknown>(
    properties: HoverCardProperties<Payload>,
): JSX.Element {
    // take the caller's card, else one of its own
    const card = controlOf(properties, () => new HoverCardControl<Payload>(properties));

    return (
        <HoverCardContext value={card}>{card.state.render(properties.children)}</HoverCardContext>
    );
}

/** Render the element that opens its hover card on hover or focus after the delay. */
export function HoverCardTrigger<Payload = unknown>(
    properties: HoverCardTriggerProperties<Payload>,
): JSX.Element {
    // open its root's card, or the one it is given from elsewhere
    const given = untrack(() => properties.control);
    const card = given ?? useHoverCard();
    const rest = omit(properties, "control", "payload");
    let element: HTMLElement | undefined;
    const change = (reason: OpenReason, target: HTMLElement, event?: Event) =>
        new OpenChange(reason, event, { element: target, payload: properties.payload });
    const hover = new Hover({
        onHoverStart: ({ target }) => card.openLater(change("trigger-hover", target)),
        onHoverEnd: (event) => card.leave(event),
    });
    onCleanup(() => given?.drop(element));

    return renderPart(
        "a",
        "hover-card-trigger",
        rest,
        [],
        mergeProperties(hover.attributes(), {
            get "data-popup-open"() {
                return card.state.isFrom(element) ? "" : undefined;
            },
            ref: (target: HTMLElement) => {
                element = target;
                if (given === undefined) {
                    card.setTrigger(target);
                }
            },
            onFocus: (event: FocusEvent & { readonly currentTarget: HTMLElement }) =>
                card.openLater(change("trigger-focus", event.currentTarget, event)),
            onBlur: (event: FocusEvent) => card.closeLater(new OpenChange("focus-out", event)),
        }),
    );
}

/** Render a preview in the top layer beside its link and open while the pointer rests on it. */
export const HoverCardContent = defineComponent({
    name: "hoverCardContent",
    properties: HOVER_CARD_CONTENT_PROPERTIES,
    render(
        properties: RenderProperties<
            HoverCardContentAttributes,
            typeof HOVER_CARD_CONTENT_PROPERTIES
        >,
    ): JSX.Element {
        // read the hover card and its placement
        const card = useHoverCard();
        const rest = omit(properties, "anchor", "side", "align", "sideOffset");

        // show and hide the card as it opens and closes
        createEffect(card.isOpen, (isOpen) => untrack(() => card.sync(isOpen)));

        // leave the card open while a pointer rests on it
        const hover = new Hover({
            onHoverStart: () => card.open(),
            onHoverEnd: () => card.closeLater(new OpenChange("trigger-hover")),
        });

        return (
            <TopLayer>
                <PositionAnchor
                    anchor={properties.anchor}
                    onAnchor={(element) => card.setAnchor(element)}
                />
                {renderPart(
                    "div",
                    "hover-card-content",
                    rest,
                    () => [
                        styles.content,
                        Position.beside(properties.side, properties.align),
                        Position.offset(properties.side, properties.sideOffset),
                    ],
                    mergeProperties(hover.attributes(), {
                        id: card.id,
                        popover: "hint" as const,
                        get "data-open"() {
                            return card.isOpen() ? "" : undefined;
                        },
                        get "data-closed"() {
                            return card.isOpen() ? undefined : "";
                        },
                        get "data-side"() {
                            return properties.side;
                        },
                        get "data-align"() {
                            return properties.align;
                        },
                        ref: (element: HTMLElement) => card.setContent(element),
                        onToggle: (event: ToggleEvent & { readonly currentTarget: HTMLElement }) =>
                            card.follow(event),
                    }),
                )}
            </TopLayer>
        );
    },
});

/** The properties of a hover card's content, the native element's attributes included. */
export type HoverCardContentProperties = ComponentProperties<typeof HoverCardContent>;
