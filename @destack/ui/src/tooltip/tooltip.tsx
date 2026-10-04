import * as style from "@destack/style";
import { color, radius, space } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import type { JSX } from "@solidjs/web";
import { createContext, merge, omit, useContext } from "solid-js";
import { Button, type ButtonProperties } from "../button/index.ts";
import {
    HoverPopover,
    placementStyle,
    type PopoverAlign,
    type PopoverSide,
} from "../popover/index.ts";
import { TopLayer } from "../layer/index.ts";

/** The wait before a resting pointer shows a tooltip, Radix's default delay in milliseconds. */
const DELAY_DURATION = 700;

/** The wait before a tooltip the pointer left hides in milliseconds, long enough to cross the gap onto it. */
const CLOSE_DELAY = 100;

/** The side and alignment of a tooltip that sets neither. */
const DEFAULTS: Required<Pick<TooltipContentProperties, "side" | "align">> = {
    side: "top",
    align: "center",
};

/** The tooltip of the nearest tooltip root, null outside one. */
export const TooltipContext = createContext<HoverPopover | null>(null);

/** The styles of a tooltip. */
const styles = style.create({
    content: {
        width: "fit-content",
        inset: { default: 0, "@supports (position-area: block-end)": "auto" },
        margin: { default: "auto", "@supports (position-area: block-end)": space[1] },
        paddingBlock: space[1],
        paddingInline: space[3],
        borderWidth: 0,
        borderRadius: radius[3],
        backgroundColor: color.foreground,
        color: color.background,
        textWrap: "balance",
    },
});

/** The properties of a tooltip root. */
export interface TooltipProperties {
    /** The wait before a resting pointer shows the tooltip in milliseconds, 700 by default. */
    readonly delayDuration?: number;
    /** The trigger and content. */
    readonly children?: JSX.Element;
}

/** The properties of a tooltip's trigger, a button's properties included. */
export type TooltipTriggerProperties = Omit<
    ButtonProperties,
    "ref" | "onPointerEnter" | "onPointerLeave" | "onFocus" | "onBlur" | "onKeyDown"
>;

/** The properties of a tooltip's content, the native element's attributes included. */
export interface TooltipContentProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "style" | "ref" | "onToggle" | "onPointerEnter" | "onPointerLeave"
> {
    /** The side of the trigger it shows on, top by default. */
    readonly side?: PopoverSide;
    /** The edge of the trigger it lines up with, center by default. */
    readonly align?: PopoverAlign;
    /** The StyleX styles applied after the tooltip's styles. */
    readonly style?: style.Styles;
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
    const tooltip = new HoverPopover(
        () => properties.delayDuration ?? DELAY_DURATION,
        () => CLOSE_DELAY,
    );

    return <TooltipContext value={tooltip}>{properties.children}</TooltipContext>;
}

/** Render a button that shows its tooltip on hover after the delay and on focus at once. */
export function TooltipTrigger(properties: TooltipTriggerProperties): JSX.Element {
    const tooltip = useTooltip();

    return (
        <Button
            data-slot="tooltip-trigger"
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
    const rest = omit(content, "side", "align", "style");

    return (
        <TopLayer>
            <div
                id={tooltip.id}
                popover="hint"
                role="tooltip"
                data-slot="tooltip-content"
                data-side={content.side}
                {...rest}
                ref={(element) => tooltip.setContent(element)}
                onToggle={(event) => tooltip.follow(event)}
                onPointerEnter={() => tooltip.open()}
                onPointerLeave={() => tooltip.closeLater()}
                {...style.attrs(
                    text.caption,
                    styles.content,
                    placementStyle(content.side, content.align),
                    content.style,
                )}
            />
        </TopLayer>
    );
}
