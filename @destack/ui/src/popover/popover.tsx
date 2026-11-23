import * as style from "@destack/style";
import { color, radius, shadow, space, stroke, weight, width } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    createEffect,
    createUniqueId,
    type JSX,
    merge,
    omit,
    onCleanup,
    Show,
    untrack,
    useContext,
} from "@destack/view";
import { transition } from "@destack/theme/motion";
import { LayerStack, TopLayer } from "../layer/index.ts";
import { Timeout } from "@destack/view/primitives/timer";
import { type HoverEvent, SafePolygon } from "../interaction/index.ts";
import { type Align, Position, type Side } from "../position/index.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import { CollectionBuilder } from "../collection/index.ts";

/** The side and alignment of a popover that sets neither. */
const DEFAULTS: Required<Pick<PopoverContentProperties, "side" | "align">> = {
    side: "bottom",
    align: "center",
};

/** The popover of the nearest popover root, null outside one. */
const PopoverContext = createContext<PopoverControl | null>(null);

/** The styles every popover shares. */
const styles = style.create({
    content: {
        width: width.popover,
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
    modal: {
        "::backdrop": {
            backgroundColor: "transparent",
        },
    },
    header: {
        display: "flex",
        flexDirection: "column",
        gap: space[1],
        marginBlockEnd: { default: space[3], ":last-child": 0 },
    },
    title: {
        fontWeight: weight.medium,
    },
    description: {
        color: color.mutedForeground,
    },
});

/** The open state of a popover, controlled or its own, and the elements that show it. */
export class PopoverControl {
    /** The id of the popover element. */
    readonly id: string;
    /** Whether the popover is open. */
    readonly isOpen: Accessor<boolean>;
    /** Whether the page behind the open popover is inert. */
    readonly isModal: Accessor<boolean>;
    /** Whether a title labels the popover in place of its trigger. */
    readonly isTitled: Accessor<boolean>;
    /** Whether a description describes the popover. */
    readonly isDescribed: Accessor<boolean>;
    /** Replace the open state and tell the change handler. */
    readonly #setOpen: (isOpen: boolean) => void;
    /** The titles on screen. */
    readonly #titles: CollectionBuilder<string>;
    /** The descriptions on screen. */
    readonly #descriptions: CollectionBuilder<string>;
    /** The button that opens the popover and takes the focus back. */
    #trigger: HTMLElement | undefined;
    /** The element the popover is placed beside in place of its trigger. */
    #anchor: HTMLElement | undefined;
    /** The popover element. */
    #content: HTMLElement | undefined;
    /** Whether the popover element is in the top layer. */
    #isShown: boolean;
    /** Stop placing the shown popover beside its trigger. */
    #unplace: () => void;
    /** Detach the popover from its anchor. */
    #detach: () => void;

    /** Create the state of a popover root, open when its properties ask for it. */
    constructor(properties: PopoverProperties) {
        // follow the controlled state, else the popover's own
        const [isOpen, setOpen] = createControllableSignal({
            isControlled: () => properties.open !== undefined,
            value: () => properties.open === true,
            defaultValue: untrack(() => properties.defaultOpen === true),
            onChange: (isNext) => properties.onOpenChange?.(isNext),
        });
        this.id = createUniqueId();
        this.isOpen = isOpen;
        this.isModal = () => properties.modal === true;
        this.isTitled = () => this.#titles.size() > 0;
        this.isDescribed = () => this.#descriptions.size() > 0;
        this.#setOpen = setOpen;
        this.#titles = new CollectionBuilder();
        this.#descriptions = new CollectionBuilder();
        this.#trigger = undefined;
        this.#anchor = undefined;
        this.#content = undefined;
        this.#isShown = false;
        this.#unplace = () => undefined;
        this.#detach = () => undefined;

        // stop placing the popover and drop its anchor once its owner goes away
        onCleanup(() => {
            this.#unplace();
            this.#detach();
        });
    }

    /** Read the popover element, undefined until it mounts. */
    protected get content(): HTMLElement | undefined {
        return this.#content;
    }

    /** Set the button that opens the popover. */
    setTrigger(element: HTMLElement): void {
        this.#trigger = element;
        this.#attach();
    }

    /** Set the element the popover is placed beside in place of its trigger. */
    setAnchor(element: HTMLElement): void {
        this.#anchor = element;
        this.#attach();
    }

    /** Label the popover by its title until the title unmounts. */
    title(): void {
        this.#titles.add(`${this.id}-title`);
    }

    /** Describe the popover by its description until the description unmounts. */
    describe(): void {
        this.#descriptions.add(`${this.id}-description`);
    }

    /** Set the popover element. */
    setContent(element: HTMLElement): void {
        this.#content = element;
        this.#attach();
    }

    /** Attach a popover the platform shows to its anchor, beside it from its first frame. */
    #attach(): void {
        // anchor a modal popover only while it shows, as its dialog opens by script
        this.#detach();
        const content = this.#content;
        const anchor = this.#anchor ?? this.#trigger;
        this.#detach =
            content === undefined || anchor === undefined || content instanceof HTMLDialogElement
                ? () => undefined
                : Position.attach(content, anchor, this.id);
    }

    /** Open the popover and tell the root's change handler. */
    open(): void {
        if (!this.isOpen()) {
            this.#setOpen(true);
        }
    }

    /** Close the popover and tell the root's change handler. */
    close(): void {
        if (this.isOpen()) {
            this.#setOpen(false);
        }
    }

    /** Follow the platform opening or closing the popover, such as on Escape or a click outside. */
    follow(event: ToggleEvent): void {
        // place a shown popover by script where the browser lacks anchor positioning, now it has a size
        this.#isShown = event.newState === "open";
        if (this.#isShown) {
            const content = this.#content;
            const anchor = this.#anchor ?? this.#trigger;
            this.#unplace =
                content === undefined || anchor === undefined
                    ? () => undefined
                    : Position.place(content, anchor);
            this.open();
        } else {
            this.#unplace();
            this.close();
        }
    }

    /** Follow the platform closing a modal popover, such as on Escape or a click outside. */
    dismiss(): void {
        if (this.#isShown) {
            this.#isShown = false;
            this.#unplace();
            this.close();
        }
    }

    /** Show the popover element anchored to its trigger, or hide it. */
    sync(isOpen: boolean): void {
        if (this.#content === undefined || isOpen === this.#isShown) {
            return;
        }

        // show a dialog element modally and lock the page behind it, else as a popover
        const content = this.#content;
        if (isOpen && content instanceof HTMLDialogElement) {
            content.showModal();
            LayerStack.of(content).open(content);
            this.#unplace = this.#holdModal(content);
        } else if (isOpen) {
            content.showPopover(
                this.#trigger === undefined ? undefined : { source: this.#trigger },
            );
        }
        // hide it the way it shows
        else {
            this.#unplace();
            if (content instanceof HTMLDialogElement) {
                content.close();
            } else {
                content.hidePopover();
            }
        }
        this.#isShown = isOpen;
    }

    /** Anchor a modal popover until it closes, leaving the stack and focusing the trigger then. */
    #holdModal(content: HTMLDialogElement): () => void {
        // anchor to the trigger, or to the anchor set in its place
        const trigger = this.#trigger;
        const anchor = this.#anchor ?? trigger;
        const unplace = anchor === undefined ? () => undefined : Position.anchor(content, anchor);

        return () => {
            // release the anchor and the layer, and hand the focus back to the trigger
            unplace();
            LayerStack.of(content).close(content);
            trigger?.focus();
        };
    }
}

/** A popover that opens while its trigger is hovered or focused. */
export class HoverPopover extends PopoverControl {
    /** The wait before a hovered trigger opens the popover, in milliseconds. */
    readonly delay: Accessor<number>;
    /** The wait before a trigger the pointer left closes the popover, in milliseconds. */
    readonly closeDelay: Accessor<number>;
    /** The pending open or close. */
    readonly #timer: Timeout;
    /** The way of a pointer from the trigger to the popover. */
    readonly #polygon: SafePolygon;

    /** Create a popover that waits the given delays and cancels them when its owner disposes. */
    constructor(
        properties: HoverPopoverProperties,
        delay: Accessor<number>,
        closeDelay: Accessor<number>,
    ) {
        // set the delays, cancelling a pending change on disposal
        super(properties);
        this.delay = delay;
        this.closeDelay = closeDelay;
        this.#timer = new Timeout();
        this.#polygon = new SafePolygon();
    }

    /** Close the popover as the pointer leaves its trigger, unless it heads for the popover. */
    leave(event: HoverEvent): void {
        this.#polygon.enter(event, this.isOpen() ? this.content : undefined);
        this.#polygon.defer(
            () => true,
            () => this.closeLater(),
        );
    }

    /** Open the popover after the open delay. */
    openLater(): void {
        this.#polygon.stop();
        this.#timer.set(() => this.open(), this.delay());
    }

    /** Close the popover after the close delay. */
    closeLater(): void {
        this.#timer.set(() => this.close(), this.closeDelay());
    }

    /** Open the popover now, cancelling a pending change. */
    override open(): void {
        this.#polygon.stop();
        this.#timer.clear();
        super.open();
    }

    /** Close the popover now, cancelling a pending change. */
    override close(): void {
        this.#polygon.stop();
        this.#timer.clear();
        super.close();
    }
}

/** The properties of a popover root. */
export interface PopoverProperties {
    /** Whether the popover is open, which makes the open state controlled. */
    readonly open?: boolean | undefined;
    /** Whether the popover starts open when its state is uncontrolled. */
    readonly defaultOpen?: boolean | undefined;
    /** Handle the popover opening or closing. */
    readonly onOpenChange?: ((open: boolean) => void) | undefined;
    /** Whether the open popover traps the focus and makes the page behind it inert. */
    readonly modal?: boolean;
    /** The trigger and content. */
    readonly children?: JSX.Element;
}

/** The properties of a popover that opens on hover, which is never modal. */
export type HoverPopoverProperties = Omit<PopoverProperties, "modal">;

/** The properties of a popover's trigger, a native button's attributes included. */
export type PopoverTriggerProperties = ElementPartProperties &
    Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "class" | "style" | "ref">;

/** The properties of a popover's content. */
export interface PopoverContentProperties extends Omit<
    JSX.HTMLAttributes<HTMLElement>,
    "class" | "ref" | "onToggle"
> {
    /** Handle a non-modal popover element opening or closing, after the popover follows it. */
    readonly onToggle?: (event: ToggleEvent & { readonly currentTarget: HTMLElement }) => void;
    /** The side of the trigger it opens on, bottom by default. */
    readonly side?: Side;
    /** The edge of the trigger it lines up with, center by default. */
    readonly align?: Align;
    /** The gap from the trigger in pixels, 4 by default. */
    readonly sideOffset?: number;
    /** The StyleX styles applied after the popover's styles. */
    readonly xstyle?: style.Styles;
}

/** Read the popover of the nearest popover root, refusing elements outside one. */
export function usePopover(): PopoverControl {
    const popover = useContext(PopoverContext);
    if (popover === null) {
        throw new TypeError("popover elements need a popover root around them");
    }

    return popover;
}

/** Connect a trigger to the popover content it opens. */
export function Popover(properties: PopoverProperties): JSX.Element {
    return (
        <PopoverContext value={new PopoverControl(properties)}>
            {properties.children}
        </PopoverContext>
    );
}

/** Render the element that toggles its popover and anchors it, a button by default. */
export function PopoverTrigger(properties: PopoverTriggerProperties): JSX.Element {
    // open a modal popover by script, else through the platform's popover target
    const popover = usePopover();

    return renderPart("button", "popover-trigger", properties, [], {
        type: "button",
        id: `${popover.id}-trigger`,
        get "data-popup-open"() {
            return popover.isOpen() ? "" : undefined;
        },
        get popovertarget() {
            return popover.isModal() ? undefined : popover.id;
        },
        "aria-haspopup": "dialog",
        get "aria-expanded"() {
            return popover.isModal() ? (popover.isOpen() ? "true" : "false") : undefined;
        },
        "aria-controls": popover.id,
        ref: (element: HTMLElement) => popover.setTrigger(element),
        onClick: () => {
            if (popover.isModal()) {
                popover.open();
            }
        },
    });
}

/** Render content in the top layer beside its trigger, labelled by the trigger. */
export function PopoverContent(properties: PopoverContentProperties): JSX.Element {
    // read the popover and its placement
    const popover = usePopover();
    const content = merge(DEFAULTS, properties);
    const rest = omit(content, "side", "align", "sideOffset", "xstyle", "style", "onToggle");
    const modalRest = omit(rest, "tabindex");

    // show and hide the popover as it opens and closes
    createEffect(popover.isOpen, (isOpen) => untrack(() => popover.sync(isOpen)));

    return (
        <TopLayer>
            <Show
                when={popover.isModal()}
                fallback={
                    <div
                        id={popover.id}
                        popover="auto"
                        role="dialog"
                        aria-labelledby={`${popover.id}-${popover.isTitled() ? "title" : "trigger"}`}
                        aria-describedby={
                            popover.isDescribed() ? `${popover.id}-description` : undefined
                        }
                        data-slot="popover-content"
                        data-open={popover.isOpen() ? "" : undefined}
                        data-closed={popover.isOpen() ? undefined : ""}
                        data-side={content.side}
                        data-align={content.align}
                        {...rest}
                        ref={(element) => popover.setContent(element)}
                        onToggle={(event) => {
                            // follow the platform before the caller's handler
                            popover.follow(event);
                            content.onToggle?.(event);
                        }}
                        {...style.attributes(
                            [
                                text.sm,
                                styles.content,
                                Position.beside(content.side, content.align),
                                Position.offset(content.side, content.sideOffset),
                                content.xstyle,
                            ],
                            content.style,
                        )}
                    />
                }
            >
                <dialog
                    id={popover.id}
                    closedby="any"
                    aria-labelledby={`${popover.id}-${popover.isTitled() ? "title" : "trigger"}`}
                    aria-describedby={
                        popover.isDescribed() ? `${popover.id}-description` : undefined
                    }
                    data-slot="popover-content"
                    data-open={popover.isOpen() ? "" : undefined}
                    data-closed={popover.isOpen() ? undefined : ""}
                    data-side={content.side}
                    data-align={content.align}
                    {...modalRest}
                    ref={(element) => popover.setContent(element)}
                    onClose={() => popover.dismiss()}
                    {...style.attributes(
                        [
                            text.sm,
                            styles.content,
                            transition.modalPopover,
                            styles.modal,
                            Position.area(content.side, content.align),
                            content.xstyle,
                        ],
                        content.style,
                    )}
                />
            </Show>
        </TopLayer>
    );
}

/** The properties of an element of a popover, the native element's attributes included. */
export type PopoverElementProperties<Target extends HTMLElement> = Omit<
    JSX.HTMLAttributes<Target>,
    "class"
> &
    ElementPartProperties;

/** Render an element the popover is placed beside in place of its trigger. */
export function PopoverAnchor(properties: PopoverElementProperties<HTMLDivElement>): JSX.Element {
    const popover = usePopover();

    return renderPart("div", "popover-anchor", properties, null, {
        ref: (element) => popover.setAnchor(element),
    });
}

/** Render the element that closes its popover, a button by default. */
export function PopoverClose(properties: PopoverTriggerProperties): JSX.Element {
    const popover = usePopover();

    return renderPart("button", "popover-close", properties, [], {
        type: "button",
        onClick: () => popover.close(),
    });
}

/** Render the top of a popover with its title and description. */
export function PopoverHeader(properties: PopoverElementProperties<HTMLDivElement>): JSX.Element {
    return renderPart("div", "popover-header", properties, styles.header);
}

/** Render the title that labels its popover. */
export function PopoverTitle(
    properties: PopoverElementProperties<HTMLHeadingElement>,
): JSX.Element {
    // label the popover for as long as the title renders
    const popover = usePopover();
    popover.title();

    return renderPart("h2", "popover-title", properties, [text.sm, styles.title], {
        get id() {
            return `${popover.id}-title`;
        },
    });
}

/** Render the description that describes its popover. */
export function PopoverDescription(
    properties: PopoverElementProperties<HTMLParagraphElement>,
): JSX.Element {
    // describe the popover for as long as the description renders
    const popover = usePopover();
    popover.describe();

    return renderPart("p", "popover-description", properties, [text.sm, styles.description], {
        get id() {
            return `${popover.id}-description`;
        },
    });
}
