import { expect, onTestFinished, test } from "@destack/test";
import { placeBeside } from "./placement.ts";

/** Give an element a fixed box in the viewport and a fixed size. */
function layOut(
    element: HTMLElement,
    box: { x: number; y: number; width: number; height: number },
): void {
    element.getBoundingClientRect = () => DOMRect.fromRect(box);
    Object.defineProperty(element, "offsetWidth", { value: box.width });
    Object.defineProperty(element, "offsetHeight", { value: box.height });
}

/** Place a 100 by 50 popover beside an anchor at a point of a 400 by 300 viewport, reading where it lands. */
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
    const stop = placeBeside(content, anchor);
    const position = [content.style.left, content.style.top, content.style.margin];
    stop();
    anchor.remove();
    content.remove();

    return [...position, content.style.left];
}

test("place a popover beside its anchor on its side and alignment, flipped and kept in the viewport, without anchor positioning", () => {
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
        // above, its end at the anchor's end, kept inside the viewport's start
        ["0px", "46px", "0px", ""],
    ]);
});
