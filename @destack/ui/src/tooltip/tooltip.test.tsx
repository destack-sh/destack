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
                render={(attributes) => <Button isIconOnly {...attributes} />}
                aria-label="Archive"
            >
                A
            </TooltipTrigger>
            <TooltipContent>Archive note</TooltipContent>
        </Tooltip>
    ));
    // focus the trigger from the keyboard, which a tooltip shows for
    const trigger = find(container, "[data-slot=tooltip-trigger]");
    document.body.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", bubbles: true }));
    trigger.focus();
    flush();
    expect(markup(container)).toBe(
        '<button data-variant="default" data-size="default" data-slot="tooltip-trigger" type="button" aria-label="Archive" data-anchor-name="--id-1" data-popup-open="" aria-describedby="id-1">A</button>' +
            '<div data-position-anchor="--id-1" data-slot="tooltip-content" id="id-1" popover="hint" role="tooltip" data-open="" data-instant="" data-side="top" data-align="center" data-popover-open="tooltip-trigger">Archive note</div>',
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
    const tooltip = () => container.querySelector("[role=tooltip]");
    const isShown = (): boolean => tooltip()?.hasAttribute("data-popover-open") === true;
    fire(trigger, "pointerenter");
    const early = isShown();
    await wait(40);
    const late = isShown();

    // move the pointer from the trigger onto the tooltip, resting past the close delay
    fire(trigger, "pointerleave");
    fire(find(container, "[role=tooltip]"), "pointerenter");
    await wait(120);
    const hovered = isShown();

    // leave the tooltip, which hides after the close delay
    fire(find(container, "[role=tooltip]"), "pointerleave");
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
    if (archive === undefined || pin === undefined) {
        throw new TypeError("the triggers did not render");
    }
    const shown = () => container.querySelector("[role=tooltip][data-popover-open]");
    fire(archive, "pointerenter");
    await wait(40);
    const first = [shown()?.textContent, shown()?.hasAttribute("data-instant")];
    fire(archive, "pointerleave");
    await wait(120);
    fire(pin, "pointerenter");
    await wait(0);
    flush();

    // the second shows at once and without motion, the first having shown after its delay
    expect([first, [shown()?.textContent, shown()?.hasAttribute("data-instant")]]).toEqual([
        ["Archive note", false],
        ["Pin note", true],
    ]);
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
        <Tooltip defaultOpen>
            <TooltipTrigger>Archive</TooltipTrigger>
            <TooltipContent sideOffset={8}>Archive note</TooltipContent>
        </Tooltip>
    ));

    expect(find(document.body, "[data-slot=tooltip-content]").getAttribute("style")).toBe(
        "--x-marginBlock: 8px;",
    );
});

test("render a tooltip's content only while it is on the page", async () => {
    stubPopovers();
    const { container } = render(() => (
        <Tooltip>
            <TooltipTrigger>Archive</TooltipTrigger>
            <TooltipContent>Archive note</TooltipContent>
        </Tooltip>
    ));
    const trigger = find(container, "[data-slot=tooltip-trigger]");
    const content = () => container.querySelector("[role=tooltip]")?.textContent;
    const before = [content(), trigger.getAttribute("aria-describedby")];
    document.body.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", bubbles: true }));
    trigger.focus();
    flush();
    const open = content();
    trigger.blur();
    flush();
    await wait(0);
    flush();

    // none before it opens, the content and its description while open, none once it left
    expect([before, open, content()]).toEqual([[undefined, null], "Archive note", undefined]);
});
