import { expect, onTestFinished, test } from "@destack/test";
import { Position } from "./position.ts";

/** Give an element a fixed box in the viewport and a fixed size. */
function layOut(
    element: HTMLElement,
    box: { x: number; y: number; width: number; height: number },
): void {
    element.getBoundingClientRect = () => DOMRect.fromRect(box);
    Object.defineProperty(element, "offsetWidth", { value: box.width });
    Object.defineProperty(element, "offsetHeight", { value: box.height });
}

/** Place a 100 by 50 popover beside an anchor at a point of a 400 by 300 viewport. */
function placed(side: string, align: string, x: number, y: number): (string | undefined)[] {
    // lay out an anchor and a popover in a document whose viewport is 400 by 300
    Object.defineProperty(document.documentElement, "clientWidth", {
        value: 400,
        configurable: true,
    });
    Object.defineProperty(document.documentElement, "clientHeight", {
        value: 300,
        configurable: true,
    });
    const anchor = document.createElement("button");
    const content = document.createElement("div");
    content.dataset["side"] = side;
    content.dataset["align"] = align;
    content.style.marginTop = "4px";
    document.body.append(anchor, content);
    layOut(anchor, { x, y, width: 40, height: 20 });
    layOut(content, { x: 0, y: 0, width: 100, height: 50 });

    // place it once, then drop the placement
    const stop = Position.place(content, anchor);
    const position = [content.style.left, content.style.top, content.style.margin];
    stop();
    anchor.remove();
    content.remove();

    return [...position, content.style.left];
}

test("place a popover beside its anchor on its side and alignment, flipped and inside the viewport, without anchor positioning", () => {
    // stand in for a browser without CSS anchor positioning
    const supports = CSS.supports.bind(CSS);
    CSS.supports = () => false;
    onTestFinished(() => {
        CSS.supports = supports;
    });

    expect([
        placed("bottom", "start", 50, 40),
        placed("bottom", "center", 50, 260),
        placed("right", "end", 340, 100),
        placed("top", "end", 10, 100),
    ]).toEqual([
        // below, its start at the anchor's start, 4 pixels from it
        ["50px", "64px", "0px", ""],
        // above, as below lacks room, centered on the anchor
        ["20px", "206px", "0px", ""],
        // left, as the right lacks room, its end at the anchor's end
        ["236px", "70px", "0px", ""],
        // above, its end at the anchor's end, inside the viewport's start
        ["0px", "46px", "0px", ""],
    ]);
});

test("share one anchor name between the overlays of one anchor, dropping it with the last of them", () => {
    // a tooltip and a popover on one button, detached one after the other
    const anchor = document.createElement("button");
    const tooltip = document.createElement("div");
    const popover = document.createElement("div");
    document.body.append(anchor, tooltip, popover);
    onTestFinished(() => {
        anchor.remove();
        tooltip.remove();
        popover.remove();
    });
    const state = () => ({
        anchor: anchor.getAttribute("data-anchor-name"),
        tooltip: tooltip.getAttribute("data-position-anchor"),
        popover: popover.getAttribute("data-position-anchor"),
        rules: document.head.querySelectorAll('style[data-anchor-rule="--tip"]').length,
    });
    const detachTooltip = Position.attach(tooltip, anchor, "tip");
    const detachPopover = Position.attach(popover, anchor, "pop");
    const attached = state();
    detachTooltip();
    const one = state();
    detachPopover();

    expect({ attached, one, none: state() }).toEqual({
        attached: { anchor: "--tip", tooltip: "--tip", popover: "--tip", rules: 1 },
        one: { anchor: "--tip", tooltip: null, popover: "--tip", rules: 1 },
        none: { anchor: null, tooltip: null, popover: null, rules: 0 },
    });
});
