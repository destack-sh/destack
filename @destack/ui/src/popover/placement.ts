/** The CSS anchor positioning property whose support places popovers without script. */
const ANCHOR_PROPERTY = "position-area";

/** The physical side a popover opens on, after the writing direction. */
type PhysicalSide = "top" | "right" | "bottom" | "left";

/** Place a shown popover beside its anchor where the browser lacks CSS anchor positioning, following scrolls and resizes, returning how to stop. */
export function placeBeside(content: HTMLElement, anchor: HTMLElement): () => void {
    // leave placement to the browser where it anchors popovers itself
    if (typeof CSS === "undefined" || CSS.supports(ANCHOR_PROPERTY, "block-end")) {
        return () => undefined;
    }

    // keep the popover's margin as its gap from the anchor
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
}

/** Anchor a shown modal popover to its anchor through a shared anchor name, as modal dialogs have no implicit anchor, returning how to stop. */
export function anchorBeside(content: HTMLElement, anchor: HTMLElement): () => void {
    // place by script where the browser lacks CSS anchor positioning
    if (typeof CSS === "undefined" || !CSS.supports(ANCHOR_PROPERTY, "block-end")) {
        return placeBeside(content, anchor);
    }

    // name the anchor after the popover and point the popover at the name
    const name = `--${content.id}`;
    anchor.style.setProperty("anchor-name", name);
    content.style.setProperty("position-anchor", name);

    return () => {
        anchor.style.removeProperty("anchor-name");
        content.style.removeProperty("position-anchor");
    };
}

/** Compute a popover's viewport position beside its anchor on its side and alignment, flipped and kept inside the viewport. */
function positionOf(content: HTMLElement, anchor: HTMLElement, gap: number): [number, number] {
    // read the anchor's box, the popover's size, the viewport and the writing direction
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
