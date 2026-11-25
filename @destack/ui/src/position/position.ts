import * as style from "@destack/style";
import { LayoutAnimation } from "../motion/index.ts";
import { transition } from "@destack/theme/motion";
import { space } from "@destack/theme/tokens.stylex";

/** The CSS anchor positioning property whose support places popovers without script. */
const ANCHOR_PROPERTY = "position-area";

/** The area whose support places popovers in their own writing direction. */
const ANCHOR_AREA = "self-block-end span-self-inline-end";

/** The gap between an overlay at a point and the edges of the screen, in pixels. */
const EDGE = 4;

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
    box: (left: string, top: string, width: string, height: string) => ({
        position: "fixed",
        left,
        top,
        width,
        height,
        pointerEvents: "none",
    }),
    point: {
        marginBlockStart: 0,
        marginBlockEnd: 0,
        marginInlineStart: 0,
        marginInlineEnd: 0,
        positionArea: "none",
    },
    at: (left: string, top: string) => ({ left, top }),
    offsetBlock: (marginBlock: string) => ({ marginBlock }),
    offsetInline: (marginInline: string) => ({ marginInline }),
});

/** The area of each side and alignment, a gap from its anchor and from the edges of the screen. */
const areas = style.create({
    "top-start": {
        marginBlock: space[1],
        marginInlineStart: 0,
        marginInlineEnd: space[1],
        positionArea: "self-block-start span-self-inline-end",
        positionTryFallbacks:
            "flip-block, flip-inline, flip-block flip-inline, self-block-start, self-block-end",
        transformOrigin: { default: "bottom left", ":dir(rtl)": "bottom right" },
    },
    "top-center": {
        marginBlock: space[1],
        marginInlineStart: space[1],
        marginInlineEnd: space[1],
        positionArea: "self-block-start",
        positionTryFallbacks: "flip-block",
        transformOrigin: "bottom center",
    },
    "top-end": {
        marginBlock: space[1],
        marginInlineStart: space[1],
        marginInlineEnd: 0,
        positionArea: "self-block-start span-self-inline-start",
        positionTryFallbacks:
            "flip-block, flip-inline, flip-block flip-inline, self-block-start, self-block-end",
        transformOrigin: { default: "bottom right", ":dir(rtl)": "bottom left" },
    },
    "right-start": {
        marginBlockStart: 0,
        marginBlockEnd: space[1],
        marginInline: space[1],
        positionArea: "self-inline-end span-self-block-end",
        positionTryFallbacks: "flip-inline",
        transformOrigin: { default: "left top", ":dir(rtl)": "right top" },
    },
    "right-center": {
        marginBlockStart: space[1],
        marginBlockEnd: space[1],
        marginInline: space[1],
        positionArea: "self-inline-end",
        positionTryFallbacks: "flip-inline",
        transformOrigin: { default: "left center", ":dir(rtl)": "right center" },
    },
    "right-end": {
        marginBlockStart: space[1],
        marginBlockEnd: 0,
        marginInline: space[1],
        positionArea: "self-inline-end span-self-block-start",
        positionTryFallbacks: "flip-inline",
        transformOrigin: { default: "left bottom", ":dir(rtl)": "right bottom" },
    },
    "bottom-start": {
        marginBlock: space[1],
        marginInlineStart: 0,
        marginInlineEnd: space[1],
        positionArea: "self-block-end span-self-inline-end",
        positionTryFallbacks:
            "flip-block, flip-inline, flip-block flip-inline, self-block-end, self-block-start",
        transformOrigin: { default: "top left", ":dir(rtl)": "top right" },
    },
    "bottom-center": {
        marginBlock: space[1],
        marginInlineStart: space[1],
        marginInlineEnd: space[1],
        positionArea: "self-block-end",
        positionTryFallbacks: "flip-block",
        transformOrigin: "top center",
    },
    "bottom-end": {
        marginBlock: space[1],
        marginInlineStart: space[1],
        marginInlineEnd: 0,
        positionArea: "self-block-end span-self-inline-start",
        positionTryFallbacks:
            "flip-block, flip-inline, flip-block flip-inline, self-block-end, self-block-start",
        transformOrigin: { default: "top right", ":dir(rtl)": "top left" },
    },
    "left-start": {
        marginBlockStart: 0,
        marginBlockEnd: space[1],
        marginInline: space[1],
        positionArea: "self-inline-start span-self-block-end",
        positionTryFallbacks: "flip-inline",
        transformOrigin: { default: "right top", ":dir(rtl)": "left top" },
    },
    "left-center": {
        marginBlockStart: space[1],
        marginBlockEnd: space[1],
        marginInline: space[1],
        positionArea: "self-inline-start",
        positionTryFallbacks: "flip-inline",
        transformOrigin: { default: "right center", ":dir(rtl)": "left center" },
    },
    "left-end": {
        marginBlockStart: space[1],
        marginBlockEnd: 0,
        marginInline: space[1],
        positionArea: "self-inline-start span-self-block-start",
        positionTryFallbacks: "flip-inline",
        transformOrigin: { default: "right bottom", ":dir(rtl)": "left bottom" },
    },
});

/** The attribute that names an anchor, which a rule turns into its anchor name. */
const ANCHOR_ATTRIBUTE = "data-anchor-name";

/** The attribute that names an overlay's anchor, which a rule turns into its position anchor. */
const ANCHORED_ATTRIBUTE = "data-position-anchor";

/** The attribute that marks the rule of an anchor's name. */
const RULE_ATTRIBUTE = "data-anchor-rule";

/** The attribute that counts the overlays sharing an anchor's rule. */
const USES_ATTRIBUTE = "data-anchor-uses";

/** A box in the viewport an overlay is placed beside, such as a text selection's. */
export type AnchorBox = Pick<DOMRectReadOnly, "x" | "y" | "width" | "height">;

/** What an overlay is placed beside in place of its trigger: an element, or a box. */
export type Anchor = HTMLElement | AnchorBox;

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

    /** Return the StyleX styles that set an overlay's gap from its anchor on a side in pixels. */
    offset(side: Side, pixels: number | undefined): style.Styles {
        // leave the gap of the area
        if (pixels === undefined) {
            return null;
        }
        const gap = `${String(pixels)}px`;

        return isBlock(side) ? styles.offsetBlock(gap) : styles.offsetInline(gap);
    },

    /** Return the StyleX styles that place an overlay at a point in the viewport. */
    at(point: Point): style.Styles {
        return [styles.point, styles.at(`${String(point.x)}px`, `${String(point.y)}px`)];
    },

    /** Move an overlay shown at a point to the point's other side where it leaves the screen. */
    fit(content: HTMLElement, point: Point): () => void {
        // leave it at the point in a page without a viewport
        const view = content.ownerDocument.documentElement;
        if (view.clientWidth === 0) {
            return () => undefined;
        }

        // open after the point in the writing direction and below it, else before and above it
        const width = content.offsetWidth;
        const height = content.offsetHeight;
        const isRightToLeft = getComputedStyle(content).direction === "rtl";
        const isBefore = isRightToLeft
            ? point.x - width >= EDGE
            : point.x + width > view.clientWidth - EDGE;
        const isAbove = point.y + height > view.clientHeight - EDGE;
        const left = isBefore ? point.x - width : point.x;
        const top = isAbove ? point.y - height : point.y;

        // write the position over the point's, away from the screen's edges
        content.style.left = `${String(clamp(left, EDGE, view.clientWidth - width - EDGE))}px`;
        content.style.top = `${String(clamp(top, EDGE, view.clientHeight - height - EDGE))}px`;

        return () => {
            content.style.removeProperty("left");
            content.style.removeProperty("top");
        };
    },

    /** Attach an overlay to its anchor by the overlay's id, where the browser anchors overlays. */
    attach(content: HTMLElement, anchor: HTMLElement, id: string): () => void {
        // leave placement to script where the browser lacks CSS anchor positioning
        if (typeof CSS === "undefined" || !CSS.supports(ANCHOR_PROPERTY, ANCHOR_AREA)) {
            return () => undefined;
        }

        // name the anchor explicitly, as an implicit anchor reaches no further than Chrome
        const name = anchor.getAttribute(ANCHOR_ATTRIBUTE) ?? `--${id}`;
        anchor.setAttribute(ANCHOR_ATTRIBUTE, name);
        content.setAttribute(ANCHORED_ATTRIBUTE, name);

        // join them by one rule per anchor in the page, counting the overlays that share it
        const selector = `style[${RULE_ATTRIBUTE}="${name}"]`;
        const rule = document.head.querySelector<HTMLStyleElement>(selector) ?? ruleOf(name);
        rule.setAttribute(USES_ATTRIBUTE, String(usesOf(rule) + 1));
        let isAttached = true;

        return () => {
            // detach once, however often the owner asks
            if (!isAttached) {
                return;
            }
            isAttached = false;

            // drop the overlay, and the anchor's name and rule with its last overlay
            content.removeAttribute(ANCHORED_ATTRIBUTE);
            const uses = usesOf(rule) - 1;
            rule.setAttribute(USES_ATTRIBUTE, String(uses));
            if (uses === 0) {
                rule.remove();
                anchor.removeAttribute(ANCHOR_ATTRIBUTE);
            }
        };
    },

    /** Place a shown overlay beside its anchor where the browser lacks CSS anchor positioning. */
    place(content: HTMLElement, anchor: HTMLElement): () => void {
        // leave placement to the browser where it anchors overlays itself
        if (typeof CSS === "undefined" || CSS.supports(ANCHOR_PROPERTY, ANCHOR_AREA)) {
            return () => undefined;
        }

        // read the overlay's margins as its gaps from its anchor and from the screen's edges
        const { marginTop, marginRight, marginBottom, marginLeft } = getComputedStyle(content);
        const side = content.dataset["side"] ?? "bottom";
        const block = Math.max(Number.parseFloat(marginTop), Number.parseFloat(marginBottom));
        const inline = Math.max(Number.parseFloat(marginLeft), Number.parseFloat(marginRight));
        const [gap, edge] = isBlock(side) ? [block, inline] : [inline, block];
        const place = (): void => {
            // write the position over the margin as inline styles
            const [left, top] = positionOf(content, anchor, gap, edge);
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

    /** Return the StyleX styles of an element that stands where a box is. */
    box({ x, y, width, height }: AnchorBox): style.Styles {
        const [left, top, across, down] = [x, y, width, height].map((each) => `${String(each)}px`);

        return styles.box(left ?? "", top ?? "", across ?? "", down ?? "");
    },

    /** Animate a shown overlay to where it now is from where it was. */
    animate(content: HTMLElement, from: DOMRect): void {
        // move at once where the person wants less motion, or the page has no animations
        if (!new LayoutAnimation(content).isEnabled()) {
            return;
        }

        // start from its old place and size
        const to = content.getBoundingClientRect();
        const way = `${String(from.left - to.left)}px ${String(from.top - to.top)}px`;
        content.animate(
            [
                {
                    translate: way,
                    width: `${String(from.width)}px`,
                    height: `${String(from.height)}px`,
                },
                {
                    translate: "0px 0px",
                    width: `${String(to.width)}px`,
                    height: `${String(to.height)}px`,
                },
            ],
            new LayoutAnimation(content).timing("short"),
        );
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

/** Report whether a side is above or below. */
function isBlock(side: string): boolean {
    return side === "top" || side === "bottom";
}

/** Compute an overlay's viewport position beside its anchor on its side and alignment. */
function positionOf(
    content: HTMLElement,
    anchor: HTMLElement,
    gap: number,
    edge: number,
): [number, number] {
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

        return [clamp(left, edge, view.clientWidth - width - edge), isBelow ? below : above];
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

    return [isAfter ? after : before, clamp(top, edge, view.clientHeight - height - edge)];
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

/** Keep a coordinate between the smallest and the largest one that fit. */
function clamp(value: number, smallest: number, largest: number): number {
    return Math.max(smallest, Math.min(value, largest));
}

/** Add the rule that turns an anchor's name into its anchor name and its overlays' anchor. */
function ruleOf(name: string): HTMLStyleElement {
    // mark the rule by the name, no overlay sharing it yet
    const rule = document.createElement("style");
    rule.setAttribute(RULE_ATTRIBUTE, name);
    rule.setAttribute(USES_ATTRIBUTE, "0");
    rule.textContent = `[${ANCHOR_ATTRIBUTE}="${name}"] { anchor-name: ${name}; } [${ANCHORED_ATTRIBUTE}="${name}"] { position-anchor: ${name}; }`;
    document.head.append(rule);

    return rule;
}

/** Count the overlays sharing an anchor's rule. */
function usesOf(rule: HTMLStyleElement): number {
    const uses = rule.getAttribute(USES_ATTRIBUTE);
    if (uses === null) {
        throw new TypeError("anchor rule is invalid: it counts no overlays");
    }

    return Number(uses);
}
