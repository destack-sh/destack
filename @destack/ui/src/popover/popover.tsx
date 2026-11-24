import * as style from "@destack/style";
import { color, radius, shadow, space, stroke, weight, width } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
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
import { tabbablesIn } from "../focus/index.ts";
import {
    Dismissal,
    LayerStack,
    OpenChange,
    type OpenProperties,
    OpenState,
    TopLayer,
} from "../layer/index.ts";
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
        outlineStyle: "none",
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
    /** The open state. */
    readonly #state: OpenState;
    /** Why the platform closes the popover. */
    readonly #dismissal: Dismissal;
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
    /** The focused element as the platform hid the popover. */
    #focused: HTMLElement | undefined;
    /** Whether the popover element is in the top layer. */
    #isShown: boolean;
    /** Stop placing the shown popover beside its trigger. */
    #unplace: () => void;
    /** Detach the popover from its anchor. */
    #detach: () => void;

    /** Create the state of a popover root, open when its properties ask for it. */
    constructor(properties: PopoverProperties) {
        // follow the controlled state, else the popover's own
        this.#state = new OpenState(properties);
        this.#dismissal = new Dismissal();
        this.id = createUniqueId();
        this.isOpen = this.#state.isOpen;
        this.isModal = () => properties.modal === true;
        this.isTitled = () => this.#titles.size() > 0;
        this.isDescribed = () => this.#descriptions.size() > 0;
        this.#titles = new CollectionBuilder();
        this.#descriptions = new CollectionBuilder();
        this.#trigger = undefined;
        this.#anchor = undefined;
        this.#content = undefined;
        this.#focused = undefined;
        this.#isShown = false;
        this.#unplace = () => undefined;
        this.#detach = () => undefined;

        // stop placing the popover and drop its anchor once its owner goes away
        onCleanup(() => {
            this.#dismissal.stop();
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

    /** Open the popover, unless its handler cancels it. */
    open(change = new OpenChange()): void {
        if (!this.isOpen()) {
            this.#state.set(true, change);
        }
    }

    /** Close the popover, unless its handler cancels it. */
    close(change = new OpenChange()): void {
        if (this.isOpen()) {
            this.#state.set(false, change);
        }
    }

    /** Refuse the platform opening the popover where its owner does. */
    request(event: ToggleEvent): void {
        // remember where the focus was before the platform hides it
        const focused = document.activeElement;
        const isInside = event.newState === "closed" && this.#content?.contains(focused) === true;
        this.#focused = isInside && focused instanceof HTMLElement ? focused : undefined;

        // leave a popover that opens by its state
        if (event.newState !== "open" || this.isOpen()) {
            return;
        }

        // open by the platform, which the state follows
        this.#isShown = true;
        if (!this.#state.follow(true, new OpenChange("trigger-press", event))) {
            this.#isShown = false;
            event.preventDefault();
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
            if (content !== undefined) {
                this.#dismissal.start(content);
                this.enter(content);
            }
        }
        // close it, or show it again where its owner refuses
        else {
            this.#unplace();
            if (this.#dismiss()) {
                this.#focused = undefined;
                this.exit();
            } else {
                this.sync(true);
            }
        }
    }

    /** Hand the focus back to the trigger, unless it moved on elsewhere. */
    protected exit(): void {
        const focused = document.activeElement;
        if (focused === document.body || this.#content?.contains(focused) === true) {
            this.#trigger?.focus({ preventScroll: true });
        }
    }

    /** Move the focus into the shown popover, where it was or to its first control. */
    protected enter(content: HTMLElement): void {
        // leave a focus already inside, as an autofocused control has
        if (!content.contains(document.activeElement)) {
            const target = this.#focused ?? tabbablesIn(content)[0] ?? content;
            target.focus({ preventScroll: true });
        }
        this.#focused = undefined;
    }

    /** Follow the platform closing the popover, reporting whether its owner took it. */
    #dismiss(): boolean {
        // read why, unless the state closed it
        const content = this.#content;
        const change =
            content === undefined
                ? new OpenChange()
                : this.#dismissal.change(content, this.#trigger);
        this.#dismissal.stop();

        return !untrack(this.isOpen) || this.#state.follow(false, change);
    }

    /** Close a modal popover as the platform asks for, such as on Escape or a press outside. */
    requestModal(event: Event & { readonly currentTarget: HTMLDialogElement }): void {
        const change = this.#dismissal.request(event);
        if (change !== undefined) {
            this.close(change);
        }
    }

    /** Follow the platform closing a modal popover, showing it again where its owner refuses. */
    followModal(): void {
        // leave one its state closed
        if (!this.#isShown) {
            this.#dismissal.stop();

            return;
        }

        // close it, or show it again
        this.#isShown = false;
        this.#unplace();
        if (!this.#dismiss()) {
            this.sync(true);
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
            this.#dismissal.start(content);
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
            () => this.closeLater(new OpenChange("trigger-hover")),
        );
    }

    /** Leave the focus on the trigger a pointer or the focus rests on. */
    protected override enter(): void {}

    /** Leave the focus where it is. */
    protected override exit(): void {}

    /** Open the popover after the open delay. */
    openLater(change?: OpenChange): void {
        this.#polygon.stop();
        this.#timer.set(() => this.open(change), this.delay());
    }

    /** Close the popover after the close delay. */
    closeLater(change?: OpenChange): void {
        this.#timer.set(() => this.close(change), this.closeDelay());
    }

    /** Open the popover now, cancelling a pending change. */
    override open(change?: OpenChange): void {
        this.#polygon.stop();
        this.#timer.clear();
        super.open(change);
    }

    /** Close the popover now, cancelling a pending change. */
    override close(change?: OpenChange): void {
        this.#polygon.stop();
        this.#timer.clear();
        super.close(change);
    }
}

/** The properties of a popover root. */
export interface PopoverProperties extends OpenProperties {
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
    "class" | "ref" | "onBeforeToggle" | "onToggle"
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
        onClick: (event: MouseEvent) => {
            if (popover.isModal()) {
                popover.open(new OpenChange("trigger-press", event));
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
    const label = (): string | undefined =>
        properties["aria-label"] === undefined
            ? `${popover.id}-${popover.isTitled() ? "title" : "trigger"}`
            : undefined;

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
                        tabindex={-1}
                        aria-labelledby={label()}
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
                        onBeforeToggle={(event) => popover.request(event)}
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
                    aria-labelledby={label()}
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
                    onCancel={(event) => popover.requestModal(event)}
                    onClose={() => popover.followModal()}
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
        onClick: (event: MouseEvent) => popover.close(new OpenChange("close-press", event)),
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
