import * as style from "@destack/style";
import { color, radius, shadow, space, stroke, weight, width } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createEffect,
    createUniqueId,
    flush,
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
    controlOf,
    OpenChange,
    type OpenProperties,
    OpenContext,
    OpenState,
    type OpenChildren,
    type OpenTrigger,
    TopLayer,
    type TriggerProperties,
} from "../layer/index.ts";
import { Timeout } from "@destack/view/primitives/timer";
import { type HoverEvent, SafePolygon } from "../interaction/index.ts";
import { type Align, type Anchor, Position, PositionAnchor, type Side } from "../position/index.ts";
import { type ElementPartProperties, renderPart, type Render } from "../part/index.ts";
import { Viewport } from "./viewport.ts";
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
    viewport: {
        position: "relative",
        overflow: "clip",
    },
});

/** The open state of a popover, controlled or its own, and the elements that show it. */
export class PopoverControl<Payload = unknown> {
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
    /** The open state, and the trigger it opened from. */
    readonly state: OpenState<Payload>;
    /** Why the platform closes the popover. */
    readonly #dismissal: Dismissal;
    /** The titles on screen. */
    readonly #titles: CollectionBuilder<string>;
    /** The descriptions on screen. */
    readonly #descriptions: CollectionBuilder<string>;
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
    /** Stop placing a modal popover. */
    #unplaceModal: () => void;
    /** The content that slides as the popover moves, undefined without a viewport. */
    #viewport: Viewport | undefined;
    /** Detach the popover from its anchor. */
    #detach: () => void;
    /** The popover element and the anchor it is attached to. */
    #attached: { readonly content: HTMLElement; readonly anchor: HTMLElement } | undefined;

    /** Create the state of a popover, open when its options ask for it. */
    constructor(options: PopoverOptions = {}) {
        // follow the controlled state, else the popover's own
        this.#dismissal = new Dismissal();
        this.id = createUniqueId();
        this.state = new OpenState(options, this.id);
        this.isOpen = this.state.isOpen;
        this.isModal = () => options.modal === true;
        this.isTitled = () => this.#titles.size() > 0;
        this.isDescribed = () => this.#descriptions.size() > 0;
        this.#titles = new CollectionBuilder();
        this.#descriptions = new CollectionBuilder();
        this.#anchor = undefined;
        this.#content = undefined;
        this.#focused = undefined;
        this.#isShown = false;
        this.#unplace = () => undefined;
        this.#unplaceModal = () => undefined;
        this.#viewport = undefined;
        this.#detach = () => undefined;
        this.#attached = undefined;

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

    /** Set the button of the popover's root. */
    setTrigger(element: HTMLElement): void {
        this.state.setTrigger(element);
        this.#attach();
    }

    /** Close the popover as the trigger it is open from goes away. */
    drop(element: HTMLElement | undefined): void {
        if (element !== undefined && untrack(() => this.isOpen() && this.state.isFrom(element))) {
            this.close(new OpenChange("none", undefined, undefined, false));
        }
    }

    /** Read the element the popover is placed beside. */
    #beside(): HTMLElement | undefined {
        return this.#anchor ?? untrack(this.state.trigger)?.element;
    }

    /** Set the element the popover is placed beside in place of its trigger. */
    setAnchor(element: HTMLElement | undefined): void {
        // attach to it
        this.#anchor = element;
        this.#attach();

        // place a shown popover there
        const content = this.#content;
        const anchor = this.#beside();
        if (this.#isShown && content !== undefined && anchor !== undefined) {
            this.#replace(content, anchor);
        }
    }

    /** Label the popover by its title until the title unmounts. */
    title(): void {
        this.#titles.add(`${this.id}-title`);
    }

    /** Describe the popover by its description until the description unmounts. */
    describe(): void {
        this.#descriptions.add(`${this.id}-description`);
    }

    /** Set the element whose content slides as the popover moves to another trigger. */
    setViewport(element: HTMLElement): void {
        this.#viewport = new Viewport(element);
    }

    /** Set the popover element. */
    setContent(element: HTMLElement): void {
        this.#content = element;
        this.#attach();
    }

    /** Attach a popover the platform shows to its anchor, beside it from its first frame. */
    #attach(): void {
        // leave one beside its anchor already
        const content = this.#content;
        const anchor = this.#beside();
        if (this.#attached?.content === content && this.#attached?.anchor === anchor) {
            return;
        }

        // anchor a modal popover only while it shows, as its dialog opens by script
        this.#detach();
        const isAnchored =
            content !== undefined &&
            anchor !== undefined &&
            !(content instanceof HTMLDialogElement);
        this.#detach = isAnchored ? Position.attach(content, anchor, this.id) : () => undefined;
        this.#attached = isAnchored ? { content, anchor } : undefined;
    }

    /** Open the popover, unless its handler cancels it. */
    open(change: OpenChange<Payload> = new OpenChange()): void {
        // move an open popover to another trigger, from where it is
        const content = this.#content;
        const isElsewhere = this.state.isElsewhere(change);
        if (this.isOpen() && isElsewhere && content !== undefined && this.#isShown) {
            const from = content.getBoundingClientRect();
            const beside = this.#beside()?.getBoundingClientRect();
            this.#viewport?.keep();
            this.state.set(true, change);
            flush();
            if (!change.isCanceled) {
                this.#move(content, from, beside);
            }
        }
        // open a closed one
        else if (!this.isOpen()) {
            this.state.set(true, change);
        }
    }

    /** Toggle the popover from a trigger, or move it there from another. */
    toggle(event: Event, trigger: OpenTrigger<Payload>): void {
        // leave the platform's own toggle, as the state shows and hides the popover
        event.preventDefault();
        const change = new OpenChange("trigger-press", event, trigger);
        if (untrack(() => this.state.isFrom(trigger.element))) {
            this.close(change);
        } else {
            this.open(change);
        }
    }

    /** Place a shown popover beside its trigger of now, gliding there from where it was. */
    #move(content: HTMLElement, from: DOMRect, beside: DOMRect | undefined): void {
        // leave one its owner placed elsewhere
        const anchor = this.#beside();
        if (anchor === undefined) {
            return;
        }

        // anchor it to the trigger of now
        this.#replace(content, anchor);

        // glide there, its content sliding the same way
        Position.animate(content, from);
        if (beside !== undefined) {
            this.#viewport?.slide(beside, anchor.getBoundingClientRect());
        }
        this.enter(content);
    }

    /** Place a shown popover beside another anchor. */
    #replace(content: HTMLElement, anchor: HTMLElement): void {
        // anchor a modal popover by its dialog, else by the platform
        if (content instanceof HTMLDialogElement) {
            this.#unplaceModal();
            this.#unplaceModal = Position.anchor(content, anchor);
        } else {
            this.#unplace();
            this.#attach();
            this.#unplace = Position.place(content, anchor);
        }
    }

    /** Close the popover, unless its handler cancels it. */
    close(change: OpenChange<Payload> = new OpenChange()): void {
        if (this.isOpen()) {
            this.state.set(false, change);
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
        const trigger = untrack(this.state.trigger);
        if (!this.state.follow(true, new OpenChange("trigger-press", event, trigger))) {
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
            const anchor = this.#beside();
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
            untrack(this.state.trigger)?.element.focus({ preventScroll: true });
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
        const trigger = untrack(this.state.trigger)?.element;
        const change =
            content === undefined
                ? new OpenChange<never>()
                : this.#dismissal.change(content, trigger);
        this.#dismissal.stop();

        return !untrack(this.isOpen) || this.state.follow(false, change);
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

    /** Mark the content as it shows or hides, where the keyboard asked for it. */
    mark(content: HTMLElement): void {
        this.state.mark(content);
    }

    /** Show the popover element anchored to its trigger, or hide it. */
    sync(isOpen: boolean): void {
        if (this.#content === undefined || isOpen === this.#isShown) {
            return;
        }

        // show a dialog element modally and lock the page behind it, else as a popover
        const content = this.#content;
        this.mark(content);
        if (isOpen && content instanceof HTMLDialogElement) {
            content.showModal();
            LayerStack.of(content).open(content);
            this.#dismissal.start(content);
            this.#unplace = this.#holdModal(content);
        }
        // show it beside its trigger
        else if (isOpen) {
            const trigger = untrack(this.state.trigger)?.element;
            this.#attach();
            content.showPopover(trigger === undefined ? undefined : { source: trigger });
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
        const anchor = this.#beside();
        this.#unplaceModal =
            anchor === undefined ? () => undefined : Position.anchor(content, anchor);

        return () => {
            // release the anchor and the layer, and hand the focus back to the trigger
            this.#unplaceModal();
            LayerStack.of(content).close(content);
            untrack(this.state.trigger)?.element.focus();
        };
    }
}

/** A popover that opens while its trigger is hovered or focused. */
export class HoverPopover<Payload = unknown> extends PopoverControl<Payload> {
    /** The wait before a hovered trigger opens the popover, in milliseconds. */
    readonly delay: Accessor<number>;
    /** The wait before a trigger the pointer left closes the popover, in milliseconds. */
    readonly closeDelay: Accessor<number>;
    /** The pending open or close. */
    readonly #timer: Timeout;
    /** The way of a pointer from the trigger to the popover. */
    readonly #polygon: SafePolygon;

    /** Create a popover that waits the given delays and cancels them when its owner disposes. */
    constructor(options: OpenProperties, delay: Accessor<number>, closeDelay: Accessor<number>) {
        // set the delays, cancelling a pending change on disposal
        super(options);
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

    /** Open the popover after the open delay, at once where it moves from another trigger. */
    openLater(change?: OpenChange<Payload>): void {
        this.#polygon.stop();
        if (untrack(this.isOpen)) {
            this.open(change);
        } else {
            this.#timer.set(() => this.open(change), this.delay());
        }
    }

    /** Close the popover after the close delay. */
    closeLater(change?: OpenChange<Payload>): void {
        this.#timer.set(() => this.close(change), this.closeDelay());
    }

    /** Open the popover now, cancelling a pending change. */
    override open(change?: OpenChange<Payload>): void {
        this.#polygon.stop();
        this.#timer.clear();
        super.open(change);
    }

    /** Close the popover now, cancelling a pending change. */
    override close(change?: OpenChange<Payload>): void {
        this.#polygon.stop();
        this.#timer.clear();
        super.close(change);
    }
}

/** How a popover opens. */
export interface PopoverOptions extends OpenProperties {
    /** Whether the open popover traps the focus and makes the page behind it inert. */
    readonly modal?: boolean;
}

/** The properties of a popover root. */
export interface PopoverProperties<Payload = unknown> extends PopoverOptions {
    /** The popover that triggers elsewhere open, which holds the open state in the root's place. */
    readonly control?: PopoverControl<Payload>;
    /** The trigger and content, or what it shows for the trigger it opened from. */
    readonly children?: OpenChildren<Payload>;
}

/** The properties of a popover's trigger, a native button's attributes included. */
export type PopoverTriggerProperties<Payload = unknown> = ElementPartProperties &
    Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "class" | "style"> &
    TriggerProperties<Payload, PopoverControl<Payload>>;

/** The properties of a popover's content. */
export interface PopoverContentProperties extends Omit<
    JSX.HTMLAttributes<HTMLElement>,
    "class" | "ref" | "onBeforeToggle" | "onToggle"
> {
    /** Handle a non-modal popover element opening or closing, after the popover follows it. */
    readonly onToggle?: (event: ToggleEvent & { readonly currentTarget: HTMLElement }) => void;
    /** What it is placed beside in place of its trigger: an element, or a box in the viewport. */
    readonly anchor?: Anchor | undefined;
    /** The side of the trigger it opens on, bottom by default. */
    readonly side?: Side;
    /** The edge of the trigger it lines up with, center by default. */
    readonly align?: Align;
    /** The gap from the trigger in pixels, 4 by default. */
    readonly sideOffset?: number;
    /** The StyleX styles applied after the popover's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
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
export function Popover<Payload = unknown>(properties: PopoverProperties<Payload>): JSX.Element {
    // take the caller's popover, else one of its own
    const control = controlOf(properties, () => new PopoverControl<Payload>(properties));

    return (
        <PopoverContext value={control}>
            <OpenContext value={control.isOpen}>
                {control.state.render(properties.children)}
            </OpenContext>
        </PopoverContext>
    );
}

/** Render the element that toggles its popover and anchors it, a button by default. */
export function PopoverTrigger<Payload = unknown>(
    properties: PopoverTriggerProperties<Payload>,
): JSX.Element {
    // open its root's popover, or the one it is given from elsewhere
    const given = untrack(() => properties.control);
    const popover = given ?? usePopover();
    const id = popover.state.triggerId();
    const rest = omit(properties, "control", "payload");
    let element: HTMLElement | undefined;
    onCleanup(() => given?.drop(element));

    return renderPart("button", "popover-trigger", rest, [], {
        type: "button",
        id,
        get "data-popup-open"() {
            return popover.state.isFrom(element) ? "" : undefined;
        },
        get popovertarget() {
            return popover.isModal() ? undefined : popover.id;
        },
        "aria-haspopup": "dialog",
        get "aria-expanded"() {
            // leave a trigger alone with its popover to the platform, which tells it open
            if (given === undefined && !popover.isModal()) {
                return undefined;
            }

            return popover.state.isFrom(element) ? "true" : "false";
        },
        "aria-controls": popover.id,
        ref: (target: HTMLElement) => {
            element = target;
            if (given === undefined) {
                popover.setTrigger(target);
            }
        },
        onClick: (event: MouseEvent & { readonly currentTarget: HTMLElement }) =>
            popover.toggle(event, { element: event.currentTarget, payload: properties.payload }),
    });
}

/** Render content in the top layer beside its trigger, labelled by the trigger. */
export function PopoverContent(properties: PopoverContentProperties): JSX.Element {
    // read the popover and its placement
    const popover = usePopover();
    const content = merge(DEFAULTS, properties);
    const rest = omit(content, "anchor", "side", "align", "sideOffset", "onToggle");
    const modalRest = omit(rest, "tabindex");
    const label = (): string | undefined =>
        properties["aria-label"] === undefined
            ? popover.isTitled()
                ? `${popover.id}-title`
                : popover.state.labelId()
            : undefined;

    // show and hide the popover as it opens and closes
    createEffect(popover.isOpen, (isOpen) => untrack(() => popover.sync(isOpen)));

    return (
        <TopLayer>
            <PositionAnchor
                anchor={content.anchor}
                onAnchor={(element) => popover.setAnchor(element)}
            />
            <Show
                when={popover.isModal()}
                fallback={renderPart(
                    "div",
                    "popover-content",
                    rest,
                    () => [
                        text.sm,
                        styles.content,
                        Position.beside(content.side, content.align),
                        Position.offset(content.side, content.sideOffset),
                    ],
                    {
                        id: popover.id,
                        popover: "auto" as const,
                        role: "dialog" as const,
                        tabindex: -1,
                        get "aria-labelledby"() {
                            return label();
                        },
                        get "aria-describedby"() {
                            return popover.isDescribed() ? `${popover.id}-description` : undefined;
                        },
                        get "data-open"() {
                            return popover.isOpen() ? "" : undefined;
                        },
                        get "data-closed"() {
                            return popover.isOpen() ? undefined : "";
                        },
                        get "data-side"() {
                            return content.side;
                        },
                        get "data-align"() {
                            return content.align;
                        },
                        ref: (element) => popover.setContent(element),
                        onBeforeToggle: (event) => popover.request(event),
                        onToggle: (event) => {
                            // follow the platform before the caller's handler
                            popover.follow(event);
                            content.onToggle?.(event);
                        },
                    },
                )}
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

/** Render content that slides as its popover moves from one trigger to another. */
export function PopoverViewport(properties: PopoverElementProperties<HTMLDivElement>): JSX.Element {
    const popover = usePopover();

    return renderPart("div", "popover-viewport", omit(properties, "children"), styles.viewport, {
        ref: (element) => popover.setViewport(element),
        get children() {
            return <div data-slot="popover-viewport-content">{properties.children}</div>;
        },
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
