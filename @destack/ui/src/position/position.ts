import * as style from "@destack/style";
import { transition } from "@destack/theme/motion";

/** The CSS anchor positioning property whose support places popovers without script. */
const ANCHOR_PROPERTY = "position-area";

/** An axis things run along. */
export type Axis = "horizontal" | "vertical";

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

/** The styles that place an overlay at a point. */
const styles = style.create({
    point: {
        margin: 0,
        positionArea: "none",
    },
    at: (left: string, top: string) => ({ left, top }),
    offset: (margin: string) => ({ margin }),
});

/** The area each side and alignment places an overlay in, growing from its anchor either way. */
const areas = style.create({
    "top-start": {
        positionArea: "block-start span-inline-end",
        positionTryFallbacks: "flip-block",
        transformOrigin: { default: "bottom left", ":dir(rtl)": "bottom right" },
    },
    "top-center": {
        positionArea: "block-start",
        positionTryFallbacks: "flip-block",
        transformOrigin: "bottom center",
    },
    "top-end": {
        positionArea: "block-start span-inline-start",
        positionTryFallbacks: "flip-block",
        transformOrigin: { default: "bottom right", ":dir(rtl)": "bottom left" },
    },
    "right-start": {
        positionArea: "inline-end span-block-end",
        positionTryFallbacks: "flip-inline",
        transformOrigin: { default: "left top", ":dir(rtl)": "right top" },
    },
    "right-center": {
        positionArea: "inline-end",
        positionTryFallbacks: "flip-inline",
        transformOrigin: { default: "left center", ":dir(rtl)": "right center" },
    },
    "right-end": {
        positionArea: "inline-end span-block-start",
        positionTryFallbacks: "flip-inline",
        transformOrigin: { default: "left bottom", ":dir(rtl)": "right bottom" },
    },
    "bottom-start": {
        positionArea: "block-end span-inline-end",
        positionTryFallbacks: "flip-block",
        transformOrigin: { default: "top left", ":dir(rtl)": "top right" },
    },
    "bottom-center": {
        positionArea: "block-end",
        positionTryFallbacks: "flip-block",
        transformOrigin: "top center",
    },
    "bottom-end": {
        positionArea: "block-end span-inline-start",
        positionTryFallbacks: "flip-block",
        transformOrigin: { default: "top right", ":dir(rtl)": "top left" },
    },
    "left-start": {
        positionArea: "inline-start span-block-end",
        positionTryFallbacks: "flip-inline",
        transformOrigin: { default: "right top", ":dir(rtl)": "left top" },
    },
    "left-center": {
        positionArea: "inline-start",
        positionTryFallbacks: "flip-inline",
        transformOrigin: { default: "right center", ":dir(rtl)": "left center" },
    },
    "left-end": {
        positionArea: "inline-start span-block-start",
        positionTryFallbacks: "flip-inline",
        transformOrigin: { default: "right bottom", ":dir(rtl)": "left bottom" },
    },
});

/** The placement of an overlay beside an anchor or at a point. */
export const Position = {
    /** Return the StyleX styles that place an overlay beside its anchor on a side and alignment. */
    beside(side: Side, align: Align): readonly style.CompiledStyles[] {
        return [transition.popover, Position.area(side, align)];
    },

    /** Return the StyleX styles of an overlay's area beside its anchor, without its motion. */
    area(side: Side, align: Align): style.CompiledStyles {
        return areas[`${side}-${align}`];
    },

    /** Return the StyleX styles that set an overlay's gap from its anchor in pixels. */
    offset(pixels: number | undefined): style.Styles {
        return pixels === undefined ? null : styles.offset(`${String(pixels)}px`);
    },

    /** Return the StyleX styles that place an overlay at a point in the viewport. */
    at(point: Point): style.Styles {
        return [styles.point, styles.at(`${String(point.x)}px`, `${String(point.y)}px`)];
    },

    /** Attach an overlay to its anchor by the overlay's id, where the browser anchors overlays. */
    attach(content: HTMLElement, anchor: HTMLElement, id: string): () => void {
        // leave placement to script where the browser lacks CSS anchor positioning
        if (typeof CSS === "undefined" || !CSS.supports(ANCHOR_PROPERTY, "block-end")) {
            return () => undefined;
        }

        // set the anchor's name explicitly, as an implicit anchor reaches no further than Chrome
        const name = `--${id}`;
        anchor.style.setProperty("anchor-name", name);
        content.style.setProperty("position-anchor", name);

        return () => {
            anchor.style.removeProperty("anchor-name");
            content.style.removeProperty("position-anchor");
        };
    },

    /** Place a shown overlay beside its anchor where the browser lacks CSS anchor positioning. */
    place(content: HTMLElement, anchor: HTMLElement): () => void {
        // leave placement to the browser where it anchors overlays itself
        if (typeof CSS === "undefined" || CSS.supports(ANCHOR_PROPERTY, "block-end")) {
            return () => undefined;
        }

        // read the overlay's margin as its gap from the anchor
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

    /** Anchor a shown overlay beside its anchor: attached where the browser can, else by script. */
    anchor(content: HTMLElement, anchor: HTMLElement): () => void {
        const detach = Position.attach(content, anchor, content.id);
        const unplace = Position.place(content, anchor);

        return () => {
            detach();
            unplace();
        };
    },
};

/** Compute an overlay's viewport position beside its anchor on its side and alignment. */
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
