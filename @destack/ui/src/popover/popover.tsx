import * as style from "@destack/style";
import {
    color,
    motion,
    radius,
    shadow,
    space,
    stroke,
    weight,
    width,
} from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    createEffect,
    createSignal,
    createUniqueId,
    type JSX,
    merge,
    omit,
    onCleanup,
    type Setter,
    Show,
    useContext,
} from "@destack/view";
import { Button, type ButtonProperties } from "../button/index.ts";
import { TopLayer } from "../layer/index.ts";
import { type Align, Position, type Side } from "../position/index.ts";

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
    header: {
        display: "flex",
        flexDirection: "column",
        gap: space[1],
    },
    title: {
        margin: 0,
        fontWeight: weight.medium,
    },
    description: {
        margin: 0,
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
    /** Whether a title names the popover in place of its trigger. */
    readonly isTitled: Accessor<boolean>;
    /** Whether a description describes the popover. */
    readonly isDescribed: Accessor<boolean>;
    /** Replace the open state and tell the change handler. */
    readonly #setOpen: (isOpen: boolean) => void;
    /** Replace whether a title names the popover. */
    readonly #setTitled: Setter<boolean>;
    /** Replace whether a description describes the popover. */
    readonly #setDescribed: Setter<boolean>;
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

    /** Create the state of a popover root, open when its properties ask for it. */
    constructor(properties: PopoverProperties) {
        // follow the controlled state, else the popover's own
        const [isOpen, setOpen] = createControllableSignal({
            isControlled: () => properties.open !== undefined,
            value: () => properties.open === true,
            defaultValue: properties.defaultOpen === true,
            onChange: (isNext) => properties.onOpenChange?.(isNext),
        });
        const [isTitled, setTitled] = createSignal(false, { ownedWrite: true });
        const [isDescribed, setDescribed] = createSignal(false, { ownedWrite: true });
        this.id = createUniqueId();
        this.isOpen = isOpen;
        this.isModal = () => properties.modal === true;
        this.isTitled = isTitled;
        this.isDescribed = isDescribed;
        this.#setOpen = setOpen;
        this.#setTitled = setTitled;
        this.#setDescribed = setDescribed;
        this.#trigger = undefined;
        this.#anchor = undefined;
        this.#content = undefined;
        this.#isShown = false;
        this.#unplace = () => undefined;
    }

    /** Set the button that opens the popover. */
    setTrigger(element: HTMLElement): void {
        this.#trigger = element;
    }

    /** Set the element the popover is placed beside in place of its trigger. */
    setAnchor(element: HTMLElement): void {
        this.#anchor = element;
    }

    /** Label the popover by its title until the title unmounts. */
    title(): void {
        this.#setTitled(true);
        onCleanup(() => this.#setTitled(false));
    }

    /** Describe the popover by its description until the description unmounts. */
    describe(): void {
        this.#setDescribed(true);
        onCleanup(() => this.#setDescribed(false));
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
            const anchor = this.#anchor ?? this.#trigger;
            content.showPopover(
                this.#trigger === undefined ? undefined : { source: this.#trigger },
            );
            this.#unplace =
                anchor === undefined ? () => undefined : Position.place(content, anchor);
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
        const anchor = this.#anchor ?? trigger;
        const unplace = anchor === undefined ? () => undefined : Position.anchor(content, anchor);
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

/** The properties of a popover root. */
export interface PopoverProperties {
    /** Whether the popover is open, which makes the open state controlled. */
    readonly open?: boolean | undefined;
    /** Whether the popover starts open when its state is uncontrolled. */
    readonly defaultOpen?: boolean | undefined;
    /** Handle the popover opening or closing. */
    readonly onOpenChange?: ((open: boolean) => void) | undefined;
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
    readonly side?: Side;
    /** The edge of the trigger it lines up with, center by default. */
    readonly align?: Align;
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
                    data-state={popover.isOpen() ? "open" : "closed"}
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
                data-state={popover.isOpen() ? "open" : "closed"}
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
                        aria-labelledby={`${popover.id}-${popover.isTitled() ? "title" : "trigger"}`}
                        aria-describedby={
                            popover.isDescribed() ? `${popover.id}-description` : undefined
                        }
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
                                Position.beside(content.side, content.align),
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
> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
};

/** Render an element the popover is placed beside in place of its trigger. */
export function PopoverAnchor(properties: PopoverElementProperties<HTMLDivElement>): JSX.Element {
    const popover = usePopover();
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            data-slot="popover-anchor"
            {...rest}
            ref={(element) => popover.setAnchor(element)}
            {...style.attributes([properties.xstyle], properties.style)}
        />
    );
}

/** Render a button that closes its popover. */
export function PopoverClose(properties: PopoverTriggerProperties): JSX.Element {
    const popover = usePopover();

    return (
        <Button
            data-slot="popover-close"
            {...properties}
            onClick={(event) => {
                // run the caller's handler before closing the popover
                properties.onClick?.(event);
                popover.close();
            }}
        />
    );
}

/** Render the top of a popover that holds its title and description. */
export function PopoverHeader(properties: PopoverElementProperties<HTMLDivElement>): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            data-slot="popover-header"
            {...rest}
            {...style.attributes([styles.header, properties.xstyle], properties.style)}
        />
    );
}

/** Render the title that names its popover. */
export function PopoverTitle(
    properties: PopoverElementProperties<HTMLHeadingElement>,
): JSX.Element {
    // label the popover for as long as the title renders
    const popover = usePopover();
    const rest = omit(properties, "xstyle", "style");
    popover.title();

    return (
        <h2
            id={`${popover.id}-title`}
            data-slot="popover-title"
            {...rest}
            {...style.attributes([text.callout, styles.title, properties.xstyle], properties.style)}
        />
    );
}

/** Render the description that describes its popover. */
export function PopoverDescription(
    properties: PopoverElementProperties<HTMLParagraphElement>,
): JSX.Element {
    // describe the popover for as long as the description renders
    const popover = usePopover();
    const rest = omit(properties, "xstyle", "style");
    popover.describe();

    return (
        <p
            id={`${popover.id}-description`}
            data-slot="popover-description"
            {...rest}
            {...style.attributes(
                [text.footnote, styles.description, properties.xstyle],
                properties.style,
            )}
        />
    );
}
