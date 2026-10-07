import * as style from "@destack/style";
import { color, motion, radius, shadow, space, stroke, width } from "@destack/theme/tokens.stylex";
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
    useContext,
} from "@destack/view";
import { Button, type ButtonProperties } from "../button/index.ts";
import { TopLayer } from "../layer/index.ts";
import { anchorBeside, placeBeside } from "./placement.ts";

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
        boxSizing: "border-box",
        width: width.popover,
        inset: "auto",
        margin: space[1],
        padding: space[4],
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[3],
        backgroundColor: color.popover,
        color: color.popoverForeground,
        boxShadow: shadow.overlay,
    },
    modal: {
        opacity: { default: 0, ":modal": { default: 1, "@starting-style": 0 } },
        transform: {
            default: "scale(0.96)",
            ":modal": { default: "none", "@starting-style": "scale(0.96)" },
        },
        transitionProperty: "opacity, transform, display, overlay",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        transitionBehavior: "allow-discrete",
        "::backdrop": {
            backgroundColor: "transparent",
        },
    },
    motion: {
        opacity: { default: 0, ":popover-open": { default: 1, "@starting-style": 0 } },
        transform: {
            default: "scale(0.96)",
            ":popover-open": { default: "none", "@starting-style": "scale(0.96)" },
        },
        transitionProperty: "opacity, transform, display, overlay",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        transitionBehavior: "allow-discrete",
    },
});

/** The area beside the anchor each side and alignment places a popover in, flipping when it overflows. */
const placements = style.create({
    "top-start": {
        positionArea: "block-start span-inline-end",
        positionTryFallbacks: "flip-block",
    },
    "top-center": { positionArea: "block-start", positionTryFallbacks: "flip-block" },
    "top-end": {
        positionArea: "block-start span-inline-start",
        positionTryFallbacks: "flip-block",
    },
    "right-start": {
        positionArea: "inline-end span-block-end",
        positionTryFallbacks: "flip-inline",
    },
    "right-center": { positionArea: "inline-end", positionTryFallbacks: "flip-inline" },
    "right-end": {
        positionArea: "inline-end span-block-start",
        positionTryFallbacks: "flip-inline",
    },
    "bottom-start": {
        positionArea: "block-end span-inline-end",
        positionTryFallbacks: "flip-block",
    },
    "bottom-center": { positionArea: "block-end", positionTryFallbacks: "flip-block" },
    "bottom-end": {
        positionArea: "block-end span-inline-start",
        positionTryFallbacks: "flip-block",
    },
    "left-start": {
        positionArea: "inline-start span-block-end",
        positionTryFallbacks: "flip-inline",
    },
    "left-center": { positionArea: "inline-start", positionTryFallbacks: "flip-inline" },
    "left-end": {
        positionArea: "inline-start span-block-start",
        positionTryFallbacks: "flip-inline",
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
    /** Replace the open state and tell the change handler. */
    readonly #setOpen: (isOpen: boolean) => void;
    /** The element that anchors the popover. */
    #trigger: HTMLElement | undefined;
    /** The popover element. */
    #content: HTMLElement | undefined;
    /** Whether the popover element is in the top layer. */
    #isShown: boolean;
    /** Stop placing the shown popover beside its trigger. */
    #unplace: () => void;

    /** Create the state of a popover root, open when its properties ask for it. */
    constructor(properties: PopoverProperties) {
        // follow the controlled state, else the popover's own
        const [isOpen, setOpen] = createControllableSignal({
            isControlled: () => properties.open !== undefined,
            value: () => properties.open === true,
            defaultValue: properties.defaultOpen === true,
            onChange: (isNext) => properties.onOpenChange?.(isNext),
        });
        this.id = createUniqueId();
        this.isOpen = isOpen;
        this.isModal = () => properties.modal === true;
        this.#setOpen = setOpen;
        this.#trigger = undefined;
        this.#content = undefined;
        this.#isShown = false;
        this.#unplace = () => undefined;
    }

    /** Set the element that anchors the popover. */
    setTrigger(element: HTMLElement): void {
        this.#trigger = element;
    }

    /** Set the popover element. */
    setContent(element: HTMLElement): void {
        this.#content = element;
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
        this.#isShown = event.newState === "open";
        if (this.#isShown) {
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
            this.#unplace = this.#holdModal(content);
        } else if (isOpen) {
            content.showPopover(
                this.#trigger === undefined ? undefined : { source: this.#trigger },
            );
            this.#unplace =
                this.#trigger === undefined ? () => undefined : placeBeside(content, this.#trigger);
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

    /** Anchor a modal popover and lock the page's scroll, returning how to release both and refocus the trigger. */
    #holdModal(content: HTMLDialogElement): () => void {
        // anchor to the trigger by name and keep the page from scrolling
        const trigger = this.#trigger;
        const unplace = trigger === undefined ? () => undefined : anchorBeside(content, trigger);
        const root = content.ownerDocument.documentElement;
        const overflow = root.style.overflow;
        root.style.overflow = "hidden";

        return () => {
            // release the page and hand the focus back to the trigger
            unplace();
            root.style.overflow = overflow;
            trigger?.focus();
        };
    }
}

/** A popover that opens while its trigger is hovered or focused, such as a tooltip or hover card. */
export class HoverPopover extends PopoverControl {
    /** The wait before a hovered trigger opens the popover, in milliseconds. */
    readonly openDelay: Accessor<number>;
    /** The wait before a trigger the pointer left closes the popover, in milliseconds. */
    readonly closeDelay: Accessor<number>;
    /** The pending open or close. */
    #timer: ReturnType<typeof setTimeout> | undefined;

    /** Create a popover that waits the given delays and cancels them when its owner disposes. */
    constructor(
        properties: HoverPopoverProperties,
        openDelay: Accessor<number>,
        closeDelay: Accessor<number>,
    ) {
        // keep the delays, cancelling a pending change on disposal
        super(properties);
        this.openDelay = openDelay;
        this.closeDelay = closeDelay;
        this.#timer = undefined;
        onCleanup(() => clearTimeout(this.#timer));
    }

    /** Open the popover after the open delay. */
    openLater(): void {
        clearTimeout(this.#timer);
        this.#timer = setTimeout(() => this.open(), this.openDelay());
    }

    /** Close the popover after the close delay. */
    closeLater(): void {
        clearTimeout(this.#timer);
        this.#timer = setTimeout(() => this.close(), this.closeDelay());
    }

    /** Open the popover now, cancelling a pending change. */
    override open(): void {
        clearTimeout(this.#timer);
        super.open();
    }

    /** Close the popover now, cancelling a pending change. */
    override close(): void {
        clearTimeout(this.#timer);
        super.close();
    }
}

/** The side of its anchor a popover opens on. */
export type PopoverSide = "top" | "right" | "bottom" | "left";

/** The edge of its anchor a popover lines up with, along its side. */
export type PopoverAlign = "start" | "center" | "end";

/** The properties of a popover root. */
export interface PopoverProperties {
    /** Whether the popover is open, which makes the open state controlled. */
    readonly open?: boolean;
    /** Whether the popover starts open when its state is uncontrolled. */
    readonly defaultOpen?: boolean;
    /** Handle the popover opening or closing. */
    readonly onOpenChange?: (open: boolean) => void;
    /** Whether the open popover traps the focus and makes the page behind it inert, false by default. */
    readonly modal?: boolean;
    /** The trigger and content. */
    readonly children?: JSX.Element;
}

/** The properties of a popover that opens on hover, which is never modal. */
export type HoverPopoverProperties = Omit<PopoverProperties, "modal">;

/** The properties of a popover's trigger, a button's properties included. */
export type PopoverTriggerProperties = Omit<ButtonProperties, "ref" | "onClick"> & {
    /** Handle a click before the button opens a modal popover. */
    readonly onClick?: (event: MouseEvent) => void;
};

/** The properties of a popover's content, the attributes of a native element or a modal dialog included. */
export interface PopoverContentProperties extends Omit<
    JSX.HTMLAttributes<HTMLElement>,
    "class" | "ref" | "onToggle"
> {
    /** Handle a non-modal popover element opening or closing, after the popover follows it. */
    readonly onToggle?: (event: ToggleEvent & { readonly currentTarget: HTMLElement }) => void;
    /** The side of the trigger it opens on, bottom by default. */
    readonly side?: PopoverSide;
    /** The edge of the trigger it lines up with, center by default. */
    readonly align?: PopoverAlign;
    /** The StyleX styles applied after the popover's styles. */
    readonly xstyle?: style.Styles;
}

/** Return the StyleX styles that place a popover beside its anchor and fade it in and out. */
export function placementStyle(
    side: PopoverSide,
    align: PopoverAlign,
): readonly style.CompiledStyles[] {
    return [styles.motion, placements[`${side}-${align}`]];
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

/** Render a button that toggles its popover, which anchors to the button. */
export function PopoverTrigger(properties: PopoverTriggerProperties): JSX.Element {
    const popover = usePopover();

    return (
        <Show
            when={popover.isModal()}
            fallback={
                <Button
                    id={`${popover.id}-trigger`}
                    data-slot="popover-trigger"
                    popovertarget={popover.id}
                    aria-haspopup="dialog"
                    aria-controls={popover.id}
                    {...properties}
                    ref={(element) => popover.setTrigger(element)}
                />
            }
        >
            <Button
                id={`${popover.id}-trigger`}
                data-slot="popover-trigger"
                aria-haspopup="dialog"
                aria-expanded={popover.isOpen() ? "true" : "false"}
                aria-controls={popover.id}
                {...properties}
                ref={(element) => popover.setTrigger(element)}
                onClick={(event) => {
                    // run the caller's handler before opening the modal popover
                    properties.onClick?.(event);
                    popover.open();
                }}
            />
        </Show>
    );
}

/** Render content in the top layer beside its trigger, named by the trigger and closed by a click outside or Escape. */
export function PopoverContent(properties: PopoverContentProperties): JSX.Element {
    // read the popover and its placement
    const popover = usePopover();
    const content = merge(DEFAULTS, properties);
    const rest = omit(content, "side", "align", "xstyle", "style", "onToggle");
    const modalRest = omit(rest, "tabindex");

    // show and hide the popover as it opens and closes
    createEffect(popover.isOpen, (isOpen) => popover.sync(isOpen));

    return (
        <TopLayer>
            <Show
                when={popover.isModal()}
                fallback={
                    <div
                        id={popover.id}
                        popover="auto"
                        role="dialog"
                        aria-labelledby={`${popover.id}-trigger`}
                        data-slot="popover-content"
                        data-state={popover.isOpen() ? "open" : "closed"}
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
                                text.callout,
                                styles.content,
                                placementStyle(content.side, content.align),
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
                    aria-labelledby={`${popover.id}-trigger`}
                    data-slot="popover-content"
                    data-state={popover.isOpen() ? "open" : "closed"}
                    data-side={content.side}
                    data-align={content.align}
                    {...modalRest}
                    ref={(element) => popover.setContent(element)}
                    onClose={() => popover.dismiss()}
                    {...style.attributes(
                        [
                            text.callout,
                            styles.content,
                            styles.modal,
                            placements[`${content.side}-${content.align}`],
                            content.xstyle,
                        ],
                        content.style,
                    )}
                />
            </Show>
        </TopLayer>
    );
}
