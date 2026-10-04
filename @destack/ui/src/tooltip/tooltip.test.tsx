import { expect, test } from "@destack/test";
import { Tooltip, TooltipContent, TooltipTrigger } from "./index.ts";
import { draw, markup, stubPopovers, wait } from "@destack/view/test";

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

test("show a tooltip at once on focus and hide it on Escape", () => {
    stubPopovers();
    const container = draw(() => (
        <Tooltip>
            <TooltipTrigger size="icon" aria-label="Archive">
                A
            </TooltipTrigger>
            <TooltipContent>Archive note</TooltipContent>
        </Tooltip>
    ));
    const trigger = find(container, "[data-slot=tooltip-trigger]");
    fire(trigger, "focus");
    const shown = markup(container);
    trigger.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));

    // the trigger is described by the tooltip, which anchors to it while shown
    expect(shown).toBe(
        '<button data-slot="tooltip-trigger" data-variant="default" data-size="icon" aria-describedby="id-1" aria-label="Archive">A</button>' +
            '<div id="id-1" popover="hint" role="tooltip" data-slot="tooltip-content" data-side="top" data-popover-open="tooltip-trigger">Archive note</div>',
    );
    expect(find(container, "[role=tooltip]").hasAttribute("data-popover-open")).toBe(false);
});

test("show a tooltip after the delay a resting pointer waits, keeping it while the pointer rests on either", async () => {
    stubPopovers();
    const container = draw(() => (
        <Tooltip delayDuration={20}>
            <TooltipTrigger>Archive</TooltipTrigger>
            <TooltipContent>Archive note</TooltipContent>
        </Tooltip>
    ));
    const trigger = find(container, "[data-slot=tooltip-trigger]");
    const tooltip = find(container, "[role=tooltip]");
    const isShown = (): boolean => tooltip.hasAttribute("data-popover-open");
    fire(trigger, "pointerenter");
    const early = isShown();
    await wait(40);
    const late = isShown();

    // move the pointer from the trigger onto the tooltip, resting past the close delay
    fire(trigger, "pointerleave");
    fire(tooltip, "pointerenter");
    await wait(120);
    const hovered = isShown();

    // leave the tooltip, which hides after the close delay
    fire(tooltip, "pointerleave");
    const leaving = isShown();
    await wait(120);
    expect([early, late, hovered, leaving, isShown()]).toEqual([false, true, true, true, false]);
});
