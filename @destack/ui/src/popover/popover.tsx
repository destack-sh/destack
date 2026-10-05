import * as style from "@destack/style";
import { color, motion, radius, shadow, space, stroke } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import type { JSX } from "@solidjs/web";
import {
    createContext,
    createUniqueId,
    merge,
    omit,
    onCleanup,
    useContext,
    type Accessor,
} from "solid-js";
import { Button, type ButtonProperties } from "../button/index.ts";
import { TopLayer } from "../layer/index.ts";

/** The width of a popover, Tailwind's w-72 that shadcn/ui's popover takes. */
const POPOVER_WIDTH = "18rem";

/** The condition under which a popover sits beside its anchor instead of the viewport's center. */
const ANCHORED = "@supports (position-area: block-end)";

/** The side and alignment of a popover that sets neither. */
const DEFAULTS: Required<Pick<PopoverContentProperties, "side" | "align">> = {
    side: "bottom",
    align: "center",
};

/** The id of the nearest popover's content, null outside a popover. */
const PopoverContext = createContext<string | null>(null);

/** The styles every popover shares. */
const styles = style.create({
    content: {
        boxSizing: "border-box",
        width: POPOVER_WIDTH,
        inset: { default: 0, [ANCHORED]: "auto" },
        margin: { default: "auto", [ANCHORED]: space[1] },
        padding: space[4],
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[3],
        backgroundColor: color.popover,
        color: color.popoverForeground,
        boxShadow: shadow.overlay,
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

/** A popover that opens while its trigger is hovered or focused, such as a tooltip or hover card. */
export class HoverPopover {
    /** The id of the popover element. */
    readonly id: string;
    /** The wait before a hovered trigger opens the popover, in milliseconds. */
    readonly openDelay: Accessor<number>;
    /** The wait before a trigger the pointer left closes the popover, in milliseconds. */
    readonly closeDelay: Accessor<number>;
    /** The element that opens the popover and anchors it. */
    #trigger: HTMLElement | undefined;
    /** The popover element. */
    #content: HTMLElement | undefined;
    /** The pending open or close. */
    #timer: ReturnType<typeof setTimeout> | undefined;
    /** Whether the popover is in the top layer. */
    #isOpen: boolean;

    /** Create a closed popover that waits the given delays and cancels them when its owner disposes. */
    constructor(openDelay: Accessor<number>, closeDelay: Accessor<number>) {
        // start closed with a fresh id, cancelling a pending change on disposal
        this.id = createUniqueId();
        this.openDelay = openDelay;
        this.closeDelay = closeDelay;
        this.#trigger = undefined;
        this.#content = undefined;
        this.#timer = undefined;
        this.#isOpen = false;
        onCleanup(() => clearTimeout(this.#timer));
    }

    /** Set the element that opens the popover and anchors it. */
    setTrigger(element: HTMLElement): void {
        this.#trigger = element;
    }

    /** Set the popover element. */
    setContent(element: HTMLElement): void {
        this.#content = element;
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

    /** Open the popover now, anchored to its trigger. */
    open(): void {
        // cancel a pending change and open a closed, mounted popover
        clearTimeout(this.#timer);
        if (this.#isOpen || this.#content === undefined || this.#trigger === undefined) {
            return;
        }
        this.#content.showPopover({ source: this.#trigger });
        this.#isOpen = true;
    }

    /** Close the popover now. */
    close(): void {
        // cancel a pending change and close an open popover
        clearTimeout(this.#timer);
        if (!this.#isOpen || this.#content === undefined) {
            return;
        }
        this.#content.hidePopover();
        this.#isOpen = false;
    }

    /** Follow the platform opening or closing the popover, such as on Escape. */
    follow(event: ToggleEvent): void {
        this.#isOpen = event.newState === "open";
    }
}

/** The side of its anchor a popover opens on. */
export type PopoverSide = "top" | "right" | "bottom" | "left";

/** The edge of its anchor a popover lines up with, along its side. */
export type PopoverAlign = "start" | "center" | "end";

/** The properties of a popover root. */
export interface PopoverProperties {
    /** The trigger and content. */
    readonly children?: JSX.Element;
}

/** The properties of a popover's content, the native element's attributes included. */
export interface PopoverContentProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "style"
> {
    /** The side of the trigger it opens on, bottom by default. */
    readonly side?: PopoverSide;
    /** The edge of the trigger it lines up with, center by default. */
    readonly align?: PopoverAlign;
    /** The StyleX styles applied after the popover's styles. */
    readonly style?: style.Styles;
}

/** Return the StyleX styles that place a popover beside its anchor and fade it in and out. */
export function placementStyle(
    side: PopoverSide,
    align: PopoverAlign,
): readonly style.CompiledStyles[] {
    return [styles.motion, placements[`${side}-${align}`]];
}

/** Read the id of the nearest popover's content, refusing elements outside a popover. */
export function usePopover(): string {
    const id = useContext(PopoverContext);
    if (id === null) {
        throw new TypeError("popover elements need a popover root around them");
    }

    return id;
}

/** Connect a trigger to the popover content it opens. */
export function Popover(properties: PopoverProperties): JSX.Element {
    return <PopoverContext value={createUniqueId()}>{properties.children}</PopoverContext>;
}

/** Render a button that toggles its popover, which anchors to the button. */
export function PopoverTrigger(properties: ButtonProperties): JSX.Element {
    const id = usePopover();

    return (
        <Button
            id={`${id}-trigger`}
            data-slot="popover-trigger"
            popovertarget={id}
            aria-haspopup="dialog"
            aria-controls={id}
            {...properties}
        />
    );
}

/** Render content in the top layer beside its trigger, named by the trigger and closed by a click outside or Escape. */
export function PopoverContent(properties: PopoverContentProperties): JSX.Element {
    // read the popover and its placement
    const id = usePopover();
    const popover = merge(DEFAULTS, properties);
    const rest = omit(popover, "side", "align", "style");

    return (
        <TopLayer>
            <div
                id={id}
                popover="auto"
                role="dialog"
                aria-labelledby={`${id}-trigger`}
                data-slot="popover-content"
                data-side={popover.side}
                data-align={popover.align}
                {...rest}
                {...style.attrs(
                    text.callout,
                    styles.content,
                    placementStyle(popover.side, popover.align),
                    popover.style,
                )}
            />
        </TopLayer>
    );
}
