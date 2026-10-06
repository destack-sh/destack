import { expect, test } from "@destack/test";
import { draw } from "@destack/view/test";
import { flush } from "@destack/view";
import {
    MessageScroller,
    MessageScrollerButton,
    MessageScrollerItem,
    MessageScrollerViewport,
} from "./index.ts";

/** Give an element a fixed scroll geometry, which the test DOM leaves at zero. */
function shape(element: HTMLElement, scrollHeight: number, clientHeight: number): void {
    Object.defineProperty(element, "scrollHeight", { configurable: true, value: scrollHeight });
    Object.defineProperty(element, "clientHeight", { configurable: true, value: clientHeight });
}

test("hide the button to the newest messages while the viewport rests at them, and show it once scrolled away", () => {
    const container = draw(() => (
        <MessageScroller>
            <MessageScrollerViewport aria-label="Conversation">
                <MessageScrollerItem>Hello</MessageScrollerItem>
            </MessageScrollerViewport>
            <MessageScrollerButton />
        </MessageScroller>
    ));
    const viewport = container.querySelector<HTMLElement>("[data-slot=message-scroller-viewport]");
    const button = container.querySelector("button");
    const atEnd = button?.inert;
    if (viewport !== null) {
        shape(viewport, 1000, 200);
        viewport.scrollTop = 100;
        viewport.dispatchEvent(new Event("scroll"));
    }
    flush();
    expect([atEnd, button?.inert, button?.getAttribute("aria-label")]).toEqual([
        true,
        false,
        "Scroll to newest",
    ]);
});
