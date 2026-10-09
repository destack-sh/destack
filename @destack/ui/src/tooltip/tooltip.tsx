import * as style from "@destack/style";
import { color, radius, space } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { createContext, createEffect, type JSX, merge, omit, useContext } from "@destack/view";
import { Button, type ButtonProperties } from "../button/index.ts";
import { HoverPopover, type HoverPopoverProperties } from "../popover/index.ts";
import { type Align, Position, type Side } from "../position/index.ts";
import { TopLayer } from "../layer/index.ts";

/** The wait before a resting pointer shows a tooltip, in milliseconds. */
const DELAY_DURATION = 700;

/** The wait before a tooltip the pointer left hides in milliseconds, long enough to cross the gap onto it. */
const CLOSE_DELAY = 100;

/** The time after a tooltip hides in which the next one shows at once, in milliseconds. */
const SKIP_DELAY_DURATION = 300;

/** The side and alignment of a tooltip that sets neither. */
const DEFAULTS: Required<Pick<TooltipContentProperties, "side" | "align">> = {
    side: "top",
    align: "center",
};

/** The tooltip of the nearest tooltip root, null outside one. */
export const TooltipContext = createContext<HoverPopover | null>(null);

/** The tooltips that show one after another without waiting, after the last one hid. */
export class TooltipGroup {
    /** The wait before a resting pointer shows a tooltip of the group, in milliseconds. */
    readonly delayDuration: number;
    /** The time after a tooltip hides in which the next one shows at once, in milliseconds. */
    readonly skipDelayDuration: number;
    /** The number of the group's tooltips on screen. */
    #shown: number;
    /** When the last of the group's tooltips hid. */
    #hiddenAt: number;

    /** Create a group that waits a delay, skipped for a while after a tooltip hides. */
    constructor(delayDuration: number, skipDelayDuration: number) {
        // start cold with no tooltip shown
        this.delayDuration = delayDuration;
        this.skipDelayDuration = skipDelayDuration;
        this.#shown = 0;
        this.#hiddenAt = Number.NEGATIVE_INFINITY;
    }

    /** Return the wait before a tooltip shows: none while another shows or just hid, else its own or the group's. */
    delayOf(own: number | undefined): number {
        const isWarm =
            this.#shown > 0 || performance.now() - this.#hiddenAt < this.skipDelayDuration;

        return isWarm ? 0 : (own ?? this.delayDuration);
    }

    /** Count a tooltip showing or hiding. */
    follow(isShown: boolean, wasShown: boolean | undefined): void {
        if (isShown) {
            this.#shown += 1;
        } else if (wasShown === true) {
            this.#shown -= 1;
            this.#hiddenAt = performance.now();
        }
    }
}

/** The group of the nearest tooltip provider, one group for the page outside one. */
const TooltipGroupContext = createContext<TooltipGroup>(
    new TooltipGroup(DELAY_DURATION, SKIP_DELAY_DURATION),
);

/** The properties of a tooltip provider. */
export interface TooltipProviderProperties {
    /** The wait before a resting pointer shows a tooltip in milliseconds, 700 by default. */
    readonly delayDuration?: number;
    /** The time after a tooltip hides in which the next one shows at once in milliseconds, 300 by default. */
    readonly skipDelayDuration?: number;
    /** The tooltips the group holds. */
    readonly children?: JSX.Element;
}

/** Group the tooltips inside, so moving from one trigger to the next shows its tooltip at once. */
export function TooltipProvider(properties: TooltipProviderProperties): JSX.Element {
    const group = new TooltipGroup(
        properties.delayDuration ?? DELAY_DURATION,
        properties.skipDelayDuration ?? SKIP_DELAY_DURATION,
    );

    return <TooltipGroupContext value={group}>{properties.children}</TooltipGroupContext>;
}

/** The styles of a tooltip. */
const styles = style.create({
    content: {
        width: "fit-content",
        inset: "auto",
        margin: space[1],
        paddingBlock: space[1],
        paddingInline: space[3],
        borderRadius: radius[3],
        backgroundColor: color.foreground,
        color: color.background,
        textWrap: "balance",
    },
});

/** The properties of a tooltip root, its open state included. */
export interface TooltipProperties extends HoverPopoverProperties {
    /** The wait before a resting pointer shows the tooltip in milliseconds, its group's by default. */
    readonly delayDuration?: number;
}

/** The properties of a tooltip's trigger, a button's properties included. */
export type TooltipTriggerProperties = Omit<
    ButtonProperties,
    "ref" | "onPointerEnter" | "onPointerLeave" | "onFocus" | "onBlur" | "onKeyDown"
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
export function useTooltip(): HoverPopover {
    const tooltip = useContext(TooltipContext);
    if (tooltip === null) {
        throw new TypeError("tooltip elements need a tooltip root around them");
    }

    return tooltip;
}

/** Connect a trigger to the tooltip that describes it. */
export function Tooltip(properties: TooltipProperties): JSX.Element {
    // wait the group's delay and tell the group as the tooltip shows and hides
    const group = useContext(TooltipGroupContext);
    const tooltip = new HoverPopover(
        properties,
        () => group.delayOf(properties.delayDuration),
        () => CLOSE_DELAY,
    );
    createEffect(tooltip.isOpen, (isShown, wasShown) => group.follow(isShown, wasShown));

    return <TooltipContext value={tooltip}>{properties.children}</TooltipContext>;
}

/** Render a button that shows its tooltip on hover after the delay and on focus at once. */
export function TooltipTrigger(properties: TooltipTriggerProperties): JSX.Element {
    const tooltip = useTooltip();

    return (
        <Button
            data-slot="tooltip-trigger"
            data-state={tooltip.isOpen() ? "open" : "closed"}
            aria-describedby={tooltip.id}
            {...properties}
            ref={(element) => tooltip.setTrigger(element)}
            onPointerEnter={() => tooltip.openLater()}
            onPointerLeave={() => tooltip.closeLater()}
            onFocus={() => tooltip.open()}
            onBlur={() => tooltip.close()}
            onKeyDown={(event) => {
                // dismiss the tooltip on Escape
                if (event.key === "Escape") {
                    tooltip.close();
                }
            }}
        />
    );
}

/** Render the text that describes its trigger, in the top layer beside it. */
export function TooltipContent(properties: TooltipContentProperties): JSX.Element {
    // read the tooltip and its placement
    const tooltip = useTooltip();
    const content = merge(DEFAULTS, properties);
    const rest = omit(content, "side", "align", "sideOffset", "xstyle", "style");

    // show and hide the tooltip as it opens and closes
    createEffect(tooltip.isOpen, (isOpen) => tooltip.sync(isOpen));

    return (
        <TopLayer>
            <div
                id={tooltip.id}
                popover="hint"
                role="tooltip"
                data-slot="tooltip-content"
                data-state={tooltip.isOpen() ? "open" : "closed"}
                data-side={content.side}
                data-align={content.align}
                {...rest}
                ref={(element) => tooltip.setContent(element)}
                onToggle={(event) => tooltip.follow(event)}
                onPointerEnter={() => tooltip.open()}
                onPointerLeave={() => tooltip.closeLater()}
                {...style.attributes(
                    [
                        text.caption,
                        styles.content,
                        Position.beside(content.side, content.align),
                        Position.offset(content.sideOffset),
                        content.xstyle,
                    ],
                    content.style,
                )}
            />
        </TopLayer>
    );
}
