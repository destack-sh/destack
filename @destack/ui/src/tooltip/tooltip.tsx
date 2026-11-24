import type { Clock } from "@destack/environment";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, radius, space, stroke } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createEffect,
    createSignal,
    type JSX,
    merge,
    omit,
    useContext,
    useEnvironment,
    untrack,
} from "@destack/view";
import { Hover } from "../interaction/index.ts";
import { type ElementPartProperties, renderPart, mergeProperties } from "../part/index.ts";
import { HoverPopover, type HoverPopoverProperties } from "../popover/index.ts";
import { type Align, Position, type Side } from "../position/index.ts";
import { OpenChange, TopLayer } from "../layer/index.ts";

/** The wait before a resting pointer shows a tooltip, in milliseconds. */
const DELAY = 700;

/** The wait before a tooltip the pointer left hides, in milliseconds. */
const CLOSE_DELAY = 100;

/** The time after a tooltip hides in which the next one shows at once, in milliseconds. */
const TIMEOUT = 300;

/** The widest a tooltip grows before its text wraps, 320px. */
const WIDTH = "20rem";

/** The side and alignment of a tooltip that sets neither. */
const DEFAULTS: Required<Pick<TooltipContentProperties, "side" | "align">> = {
    side: "top",
    align: "center",
};

/** A tooltip: a hover popover that shows at once and without motion while its group is warm. */
export class TooltipControl extends HoverPopover {
    /** Whether the tooltip shows without motion: warm, or opened by the keyboard. */
    readonly isInstant: Accessor<boolean>;
    /** The group the tooltip shows and hides in. */
    readonly #group: TooltipGroup;
    /** The clock the group's warmth is read on. */
    readonly #clock: Clock;
    /** Set whether the tooltip shows without motion. */
    readonly #setInstant: (isInstant: boolean) => void;

    /** Create a tooltip of a group, waiting the group's delay on a clock. */
    constructor(properties: TooltipProperties, group: TooltipGroup, clock: Clock) {
        // wait the group's delay, none while it is warm
        super(
            properties,
            () => group.delayOf(properties.delay, clock.now()),
            () => properties.closeDelay ?? group.closeDelay,
        );
        this.#group = group;
        this.#clock = clock;
        const [isInstant, setInstant] = createSignal(false);
        this.isInstant = isInstant;
        this.#setInstant = setInstant;
    }

    /** Open after the delay, at once and without motion while the group is warm. */
    override openLater(change?: OpenChange): void {
        this.#setInstant(this.#group.isWarm(this.#clock.now()));
        super.openLater(change);
    }

    /** Open now, without motion where asked, as when the keyboard reaches the trigger. */
    override open(change?: OpenChange, isImmediate = false): void {
        if (isImmediate) {
            this.#setInstant(true);
        }
        super.open(change);
    }
}

/** The tooltip of the nearest tooltip root, null outside one. */
export const TooltipContext = createContext<TooltipControl | null>(null);

/** The tooltips that show one after another without waiting, after the last one hid. */
export class TooltipGroup {
    /** The wait before a resting pointer shows a tooltip of the group, in milliseconds. */
    readonly delay: number;
    /** The wait before a tooltip of the group the pointer left hides, in milliseconds. */
    readonly closeDelay: number;
    /** The time after a tooltip hides in which the next one shows at once, in milliseconds. */
    readonly timeout: number;
    /** The number of the group's tooltips on screen. */
    #shown: number;
    /** When the last of the group's tooltips hid. */
    #hiddenAt: number;

    /** Create a group that waits a delay, skipped for a while after a tooltip hides. */
    constructor(delay: number, closeDelay: number, timeout: number) {
        // start cold with no tooltip shown
        this.delay = delay;
        this.closeDelay = closeDelay;
        this.timeout = timeout;
        this.#shown = 0;
        this.#hiddenAt = Number.NEGATIVE_INFINITY;
    }

    /** Report whether a tooltip of the group showed a moment ago, so the next shows at once. */
    isWarm(now: number): boolean {
        return this.#shown > 0 || now - this.#hiddenAt < this.timeout;
    }

    /** Return the wait before a tooltip shows at a time. */
    delayOf(own: number | undefined, now: number): number {
        return this.isWarm(now) ? 0 : (own ?? this.delay);
    }

    /** Count a tooltip showing or hiding at a time. */
    follow(isShown: boolean, wasShown: boolean | undefined, now: number): void {
        if (isShown) {
            this.#shown += 1;
        } else if (wasShown === true) {
            this.#shown -= 1;
            this.#hiddenAt = now;
        }
    }
}

/** The group of the nearest tooltip provider, one group for the page outside one. */
const TooltipGroupContext = createContext<TooltipGroup>(
    new TooltipGroup(DELAY, CLOSE_DELAY, TIMEOUT),
);

/** The properties of a tooltip provider. */
export interface TooltipProviderProperties {
    /** The wait before a resting pointer shows a tooltip in milliseconds, 700 by default. */
    readonly delay?: number;
    /** The wait before a tooltip the pointer left hides in milliseconds, 100 by default. */
    readonly closeDelay?: number;
    /** The time after a tooltip hides in which the next one shows at once. */
    readonly timeout?: number;
    /** The tooltips the group includes. */
    readonly children?: JSX.Element;
}

/** Group the tooltips inside, so moving from one trigger to the next shows its tooltip at once. */
export function TooltipProvider(properties: TooltipProviderProperties): JSX.Element {
    const group = new TooltipGroup(
        properties.delay ?? DELAY,
        properties.closeDelay ?? CLOSE_DELAY,
        properties.timeout ?? TIMEOUT,
    );

    return <TooltipGroupContext value={group}>{properties.children}</TooltipGroupContext>;
}

/** The styles of a tooltip. */
const styles = style.create({
    content: {
        outlineStyle: { default: "none", [media.forcedColors]: "solid" },
        outlineWidth: stroke.border,
        width: "fit-content",
        maxWidth: `min(${WIDTH}, 100vw - 2 * ${space[1]})`,
        inset: "auto",
        paddingBlock: space[1],
        paddingInline: space[3],
        borderRadius: radius[3],
        backgroundColor: color.foreground,
        color: color.background,
        textWrap: "balance",
        overflowWrap: "anywhere",
    },
    instant: {
        transitionDuration: "0s",
    },
});

/** The properties of a tooltip root, its open state included. */
export interface TooltipProperties extends HoverPopoverProperties {
    /** The wait before a resting pointer shows the tooltip, in milliseconds. */
    readonly delay?: number;
    /** The wait before a tooltip the pointer left hides, in milliseconds. */
    readonly closeDelay?: number;
}

/** The properties of a tooltip's trigger, a native button's attributes included. */
export type TooltipTriggerProperties = ElementPartProperties &
    Omit<
        JSX.ButtonHTMLAttributes<HTMLButtonElement>,
        "class" | "style" | "ref" | "onPointerEnter" | "onPointerLeave"
    >;

/** The properties of a tooltip's content, the native element's attributes included. */
export interface TooltipContentProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "ref" | "onToggle" | "onPointerEnter" | "onPointerLeave"
> {
    /** The side of the trigger it shows on, top by default. */
    readonly side?: Side;
    /** The edge of the trigger it lines up with, center by default. */
    readonly align?: Align;
    /** The gap from the trigger in pixels, 4 by default. */
    readonly sideOffset?: number;
    /** The StyleX styles applied after the tooltip's styles. */
    readonly xstyle?: style.Styles;
}

/** Read the tooltip of the nearest tooltip root, refusing elements outside one. */
export function useTooltip(): TooltipControl {
    const tooltip = useContext(TooltipContext);
    if (tooltip === null) {
        throw new TypeError("tooltip elements need a tooltip root around them");
    }

    return tooltip;
}

/** Connect a trigger to the tooltip that describes it. */
export function Tooltip(properties: TooltipProperties): JSX.Element {
    // count the tooltip in its group as it shows and hides, on the page's clock
    const group = useContext(TooltipGroupContext);
    const { clock } = useEnvironment();
    const tooltip = new TooltipControl(properties, group, clock);
    createEffect(tooltip.isOpen, (isShown, wasShown) =>
        group.follow(isShown, wasShown, clock.now()),
    );

    return <TooltipContext value={tooltip}>{properties.children}</TooltipContext>;
}

/** Render the element that shows its tooltip on hover after the delay and on focus at once. */
export function TooltipTrigger(properties: TooltipTriggerProperties): JSX.Element {
    // open the tooltip on hover after its delay, and on focus at once
    const tooltip = useTooltip();
    let isPressed = false;
    const hover = new Hover({
        onHoverStart: () => tooltip.openLater(new OpenChange("trigger-hover")),
        onHoverEnd: (event) => {
            isPressed = false;
            tooltip.leave(event);
        },
    });

    return renderPart(
        "button",
        "tooltip-trigger",
        properties,
        [],
        mergeProperties(hover.attributes(), {
            type: "button" as const,
            get "data-popup-open"() {
                return tooltip.isOpen() ? "" : undefined;
            },
            "aria-describedby": tooltip.id,
            ref: (element: HTMLElement) => tooltip.setTrigger(element),
            onFocus: (event: FocusEvent & { readonly currentTarget: HTMLElement }) => {
                // show a tooltip the keyboard reaches at once and without motion, none for a press
                if (!isPressed) {
                    const isImmediate = event.currentTarget.matches(":focus-visible");
                    tooltip.open(new OpenChange("trigger-focus", event), isImmediate);
                }
            },
            onPointerDown: (event: PointerEvent) => {
                isPressed = true;
                tooltip.close(new OpenChange("trigger-press", event));
            },
            onPointerUp: () => (isPressed = false),
            onPointerCancel: () => (isPressed = false),
            onBlur: (event: FocusEvent) => tooltip.close(new OpenChange("focus-out", event)),
            onKeyDown: (event: KeyboardEvent) => {
                // dismiss the tooltip on Escape
                if (event.key === "Escape") {
                    tooltip.close(new OpenChange("escape-key", event));
                }
            },
        }),
    );
}

/** Render the text that describes its trigger, in the top layer beside it. */
export function TooltipContent(properties: TooltipContentProperties): JSX.Element {
    // read the tooltip and its placement
    const tooltip = useTooltip();
    const isInstant = tooltip.isInstant;
    const content = merge(DEFAULTS, properties);
    const rest = omit(content, "side", "align", "sideOffset", "xstyle", "style");

    // show and hide the tooltip as it opens and closes
    createEffect(tooltip.isOpen, (isOpen) => untrack(() => tooltip.sync(isOpen)));

    // leave the tooltip open while a pointer rests on it
    const hover = new Hover({
        onHoverStart: () => tooltip.open(),
        onHoverEnd: () => tooltip.closeLater(new OpenChange("trigger-hover")),
    });

    return (
        <TopLayer>
            <div
                id={tooltip.id}
                popover="hint"
                role="tooltip"
                data-slot="tooltip-content"
                data-open={tooltip.isOpen() ? "" : undefined}
                data-closed={tooltip.isOpen() ? undefined : ""}
                data-instant={isInstant() ? "" : undefined}
                data-side={content.side}
                data-align={content.align}
                {...rest}
                ref={(element) => tooltip.setContent(element)}
                onToggle={(event) => tooltip.follow(event)}
                {...hover.attributes()}
                {...style.attributes(
                    [
                        text.xs,
                        styles.content,
                        Position.beside(content.side, content.align),
                        Position.offset(content.side, content.sideOffset),
                        isInstant() && styles.instant,
                        content.xstyle,
                    ],
                    content.style,
                )}
            />
        </TopLayer>
    );
}
