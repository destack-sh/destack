import * as style from "@destack/style";
import { motion } from "@destack/theme/tokens.stylex";

/** The CSS anchor positioning property whose support places popovers without script. */
const ANCHOR_PROPERTY = "position-area";

/** The side of its anchor an overlay opens on. */
export type Side = "top" | "right" | "bottom" | "left";

/** The edge of its anchor an overlay lines up with, along its side. */
export type Align = "start" | "center" | "end";

/** A point in the viewport an overlay opens at, in CSS pixels. */
export interface Point {
    /** The distance from the viewport's left edge. */
    readonly x: number;
    /** The distance from the viewport's top edge. */
    readonly y: number;
}

/** The physical side an overlay opens on, after the writing direction. */
type PhysicalSide = "top" | "right" | "bottom" | "left";

/** The styles that fade an overlay in and out and place it at a point. */
const styles = style.create({
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
    point: {
        margin: 0,
        positionArea: "none",
    },
    at: (left: string, top: string) => ({ left, top }),
});

/** The area beside the anchor each side and alignment places an overlay in, flipping when it overflows. */
const areas = style.create({
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

/** The placement of an overlay beside an anchor or at a point. */
export const Position = {
    /** Return the StyleX styles that place an overlay beside its anchor on a side and alignment and fade it in and out. */
    beside(side: Side, align: Align): readonly style.CompiledStyles[] {
        return [styles.motion, Position.area(side, align)];
    },

    /** Return the StyleX styles that place an overlay beside its anchor on a side and alignment, such as a modal dialog that fades itself. */
    area(side: Side, align: Align): style.CompiledStyles {
        return areas[`${side}-${align}`];
    },

    /** Return the StyleX styles that place an overlay at a point in the viewport. */
    at(point: Point): style.Styles {
        return [styles.point, styles.at(`${String(point.x)}px`, `${String(point.y)}px`)];
    },

    /** Place a shown overlay beside its anchor where the browser lacks CSS anchor positioning, following scrolls and resizes, returning how to stop. */
    place(content: HTMLElement, anchor: HTMLElement): () => void {
        // leave placement to the browser where it anchors popovers itself
        if (typeof CSS === "undefined" || CSS.supports(ANCHOR_PROPERTY, "block-end")) {
            return () => undefined;
        }

        // keep the overlay's margin as its gap from the anchor
        const gap = Number.parseFloat(getComputedStyle(content).marginTop);
        const place = (): void => {
            // write the position over the margin as inline styles
            const [left, top] = positionOf(content, anchor, gap);
            content.style.margin = "0";
            content.style.left = `${String(left)}px`;
            content.style.top = `${String(top)}px`;
        };

        // place it at once and on every scroll and resize
        place();
        const view = content.ownerDocument.defaultView ?? window;
        view.addEventListener("scroll", place, { capture: true, passive: true });
        view.addEventListener("resize", place, { passive: true });

        // stop following and drop the placement
        return () => {
            view.removeEventListener("scroll", place, { capture: true });
            view.removeEventListener("resize", place);
            for (const property of ["margin", "left", "top"]) {
                content.style.removeProperty(property);
            }
        };
    },

    /** Anchor a shown modal overlay to its anchor through a shared anchor name, as modal dialogs have no implicit anchor, returning how to stop. */
    anchor(content: HTMLElement, anchor: HTMLElement): () => void {
        // place by script where the browser lacks CSS anchor positioning
        if (typeof CSS === "undefined" || !CSS.supports(ANCHOR_PROPERTY, "block-end")) {
            return Position.place(content, anchor);
        }

        // set the anchor's name from the overlay's id and point the overlay at it
        const name = `--${content.id}`;
        anchor.style.setProperty("anchor-name", name);
        content.style.setProperty("position-anchor", name);

        return () => {
            anchor.style.removeProperty("anchor-name");
            content.style.removeProperty("position-anchor");
        };
    },
};

/** Compute an overlay's viewport position beside its anchor on its side and alignment, flipped and kept inside the viewport. */
function positionOf(content: HTMLElement, anchor: HTMLElement, gap: number): [number, number] {
    // read the anchor's box, the overlay's size, the viewport and the writing direction
    const box = anchor.getBoundingClientRect();
    const width = content.offsetWidth;
    const height = content.offsetHeight;
    const view = content.ownerDocument.documentElement;
    const isRightToLeft = getComputedStyle(anchor).direction === "rtl";
    const side = physicalSide(content.dataset["side"] ?? "bottom", isRightToLeft);
    const align = content.dataset["align"] ?? "center";

    // open above or below, flipped when the side lacks room, lined up along the inline axis
    if (side === "top" || side === "bottom") {
        const below = box.bottom + gap;
        const above = box.top - gap - height;
        const isBelow =
            side === "bottom" ? below + height <= view.clientHeight || above < 0 : above < 0;
        const start = isRightToLeft ? box.right - width : box.left;
        const end = isRightToLeft ? box.left : box.right - width;
        const left =
            align === "start" ? start : align === "end" ? end : box.left + (box.width - width) / 2;

        return [clamp(left, view.clientWidth - width), isBelow ? below : above];
    }

    // open beside, flipped when the side lacks room, lined up along the block axis
    const after = box.right + gap;
    const before = box.left - gap - width;
    const isAfter = side === "right" ? after + width <= view.clientWidth || before < 0 : before < 0;
    const top =
        align === "start"
            ? box.top
            : align === "end"
              ? box.bottom - height
              : box.top + (box.height - height) / 2;

    return [isAfter ? after : before, clamp(top, view.clientHeight - height)];
}

/** Read the physical side of a logical one, right and left swapping in right-to-left text. */
function physicalSide(side: string, isRightToLeft: boolean): PhysicalSide {
    // swap the inline sides in right-to-left text
    if (side === "right") {
        return isRightToLeft ? "left" : "right";
    } else if (side === "left") {
        return isRightToLeft ? "right" : "left";
    } else if (side === "top") {
        return "top";
    }

    return "bottom";
}

/** Keep a coordinate between the viewport's start and the largest one that fits. */
function clamp(value: number, largest: number): number {
    return Math.max(0, Math.min(value, largest));
}
