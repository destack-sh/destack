import { expect, test } from "@destack/test";
import { HoverCard, HoverCardContent, HoverCardTrigger } from "./index.ts";
import { draw, stubPopovers, wait } from "@destack/view/test";

/** Find the element of a container's first match, refusing none. */
function find(container: Element, selector: string): HTMLElement {
    const element = container.querySelector<HTMLElement>(selector);
    if (element === null) {
        throw new TypeError(`no element matches ${selector}`);
    }

    return element;
}

/** Dispatch a pointer or focus event on an element. */
function fire(element: Element, type: string): void {
    element.dispatchEvent(
        new Event(type, { bubbles: type !== "pointerenter" && type !== "pointerleave" }),
    );
}

test("keep a hover card open while the pointer moves from its link onto it", async () => {
    stubPopovers();
    const container = draw(() => (
        <HoverCard openDelay={10} closeDelay={20}>
            <HoverCardTrigger href="/people/ada">@ada</HoverCardTrigger>
            <HoverCardContent>Ada Lovelace, mathematician</HoverCardContent>
        </HoverCard>
    ));
    const link = find(container, "a");
    const card = find(container, "[data-slot=hover-card-content]");
    fire(link, "pointerenter");
    await wait(30);
    const opened = card.getAttribute("data-popover-open");
    fire(link, "pointerleave");
    fire(card, "pointerenter");
    await wait(40);
    const kept = card.hasAttribute("data-popover-open");
    fire(card, "pointerleave");
    await wait(40);

    // the card anchors to the link, stays while hovered and closes after the pointer leaves it
    expect([opened, kept, card.hasAttribute("data-popover-open")]).toEqual([
        "hover-card-trigger",
        true,
        false,
    ]);
});
