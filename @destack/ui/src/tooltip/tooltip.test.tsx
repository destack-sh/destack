import { Button } from "../button/index.ts";
import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "./index.ts";
import { markup, render, stubPopovers, wait } from "@destack/view/test";

/** Find the element of a container's first match, refusing none. */
function find(container: Element, selector: string): HTMLElement {
    const element = container.querySelector<HTMLElement>(selector);
    if (element === null) {
        throw new TypeError(`no element matches '${selector}'`);
    }

    return element;
}

/** Dispatch a pointer or focus event on an element. */
function fire(element: Element, type: string): void {
    element.dispatchEvent(
        new Event(type, { bubbles: type !== "pointerenter" && type !== "pointerleave" }),
    );
}

test("describe a trigger by its tooltip, anchored to it while shown", () => {
    stubPopovers();
    const { container } = render(() => (
        <Tooltip>
            <TooltipTrigger
                render={(attributes) => <Button {...attributes} size="icon" />}
                aria-label="Archive"
            >
                A
            </TooltipTrigger>
            <TooltipContent>Archive note</TooltipContent>
        </Tooltip>
    ));
    const trigger = find(container, "[data-slot=tooltip-trigger]");
    fire(trigger, "focus");
    flush();
    expect(markup(container)).toBe(
        '<button data-slot="tooltip-trigger" data-variant="default" data-size="icon" type="button" aria-describedby="id-1" aria-label="Archive" data-anchor-name="--id-1" data-popup-open="">A</button>' +
            '<div data-position-anchor="--id-1" id="id-1" popover="hint" role="tooltip" data-slot="tooltip-content" data-side="top" data-align="center" data-open="" data-popover-open="tooltip-trigger">Archive note</div>',
    );
});

test("show a tooltip after the delay a resting pointer waits, and while the pointer rests on either", async () => {
    stubPopovers();
    const { container } = render(() => (
        <Tooltip delay={20}>
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

test("show the next tooltip of a group at once and without motion while the last one just hid", async () => {
    stubPopovers();
    const { container } = render(() => (
        <TooltipProvider delay={20} timeout={200}>
            <Tooltip>
                <TooltipTrigger>Archive</TooltipTrigger>
                <TooltipContent>Archive note</TooltipContent>
            </Tooltip>
            <Tooltip>
                <TooltipTrigger>Pin</TooltipTrigger>
                <TooltipContent>Pin note</TooltipContent>
            </Tooltip>
        </TooltipProvider>
    ));
    const [archive, pin] = [...container.querySelectorAll("[data-slot=tooltip-trigger]")];
    const [archiveTooltip, pinTooltip] = [...container.querySelectorAll("[role=tooltip]")];
    if (
        archive === undefined ||
        pin === undefined ||
        archiveTooltip === undefined ||
        pinTooltip === undefined
    ) {
        throw new TypeError("the tooltips did not render");
    }
    fire(archive, "pointerenter");
    await wait(40);
    fire(archive, "pointerleave");
    await wait(120);
    fire(pin, "pointerenter");
    await wait(0);
    flush();

    // the second shows at once and without motion, the first having shown after its delay with it
    expect({
        isShown: pinTooltip.hasAttribute("data-popover-open"),
        isInstant: [
            archiveTooltip.hasAttribute("data-instant"),
            pinTooltip.hasAttribute("data-instant"),
        ],
    }).toEqual({ isShown: true, isInstant: [false, true] });
});

test("mark its trigger hovered while a mouse rests on it, and not once it leaves", () => {
    stubPopovers();
    const { container } = render(() => (
        <Tooltip>
            <TooltipTrigger>A</TooltipTrigger>
            <TooltipContent>Archive note</TooltipContent>
        </Tooltip>
    ));
    const trigger = find(container, "[data-slot=tooltip-trigger]");
    const pointer = (type: string) => {
        trigger.dispatchEvent(new PointerEvent(type, { pointerType: "mouse" }));
        flush();

        return trigger.hasAttribute("data-hovered");
    };

    expect([pointer("pointerenter"), pointer("pointerleave")]).toEqual([true, false]);
});

test("set a tooltip's gap from its trigger", () => {
    stubPopovers();
    render(() => (
        <Tooltip>
            <TooltipTrigger>Archive</TooltipTrigger>
            <TooltipContent sideOffset={8}>Archive note</TooltipContent>
        </Tooltip>
    ));

    expect(find(document.body, "[data-slot=tooltip-content]").getAttribute("style")).toBe(
        "--x-margin: 8px;",
    );
});
