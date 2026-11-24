import { LayerStack } from "../layer/index.ts";
import { expect, test } from "@destack/test";
import {
    Popover,
    PopoverAnchor,
    PopoverClose,
    PopoverContent,
    PopoverDescription,
    PopoverTitle,
    PopoverTrigger,
} from "./index.ts";
import { flush } from "@destack/view";
import { markup, render } from "@destack/view/test";

test("toggle a popover from its trigger through the platform's popover target, anchored to the trigger by name", () => {
    const { container } = render(() => (
        <Popover>
            <PopoverTrigger>Share</PopoverTrigger>
            <PopoverContent align="start">Anyone with the link can view.</PopoverContent>
        </Popover>
    ));
    expect(markup(container)).toBe(
        '<button data-slot="popover-trigger" type="button" id="id-1-trigger" popovertarget="id-1" aria-haspopup="dialog" aria-controls="id-1" data-anchor-name="--id-1">Share</button>' +
            '<div id="id-1" popover="auto" role="dialog" tabindex="-1" aria-labelledby="id-1-trigger" data-slot="popover-content" data-closed="" data-side="bottom" data-align="start" data-position-anchor="--id-1">Anyone with the link can view.</div>',
    );
});

test("move the focus into a shown popover, to its first control, unless one has it", () => {
    // show a popover with controls, one without, and one whose second control took the focus
    const { container } = render(() => (
        <>
            <Popover>
                <PopoverContent id="controls">
                    <button type="button">First</button>
                </PopoverContent>
            </Popover>
            <Popover>
                <PopoverContent id="text">Nothing to press</PopoverContent>
            </Popover>
            <Popover>
                <PopoverContent id="taken">
                    <button type="button">First</button>
                    <button type="button">Second</button>
                </PopoverContent>
            </Popover>
        </>
    ));
    flush();
    const focused = ["controls", "text", "taken"].map((id) => {
        const content = container.ownerDocument.getElementById(id);
        if (id === "taken") {
            content?.querySelector<HTMLElement>("button:last-child")?.focus();
        }
        content?.dispatchEvent(
            Object.assign(new Event("toggle"), { newState: "open", oldState: "closed" }),
        );
        flush();

        return document.activeElement?.textContent;
    });

    expect(focused).toEqual(["First", "Nothing to press", "Second"]);
});

test("hand the focus back to the trigger as a popover closes, unless it moved on", () => {
    const { container } = render(() => (
        <>
            <Popover>
                <PopoverTrigger>Share</PopoverTrigger>
                <PopoverContent id="share">
                    <button type="button">Inside</button>
                </PopoverContent>
            </Popover>
            <button type="button">Elsewhere</button>
        </>
    ));
    flush();
    const content = container.ownerDocument.getElementById("share");
    const toggle = (newState: string) => {
        content?.dispatchEvent(Object.assign(new Event("toggle"), { newState }));
        flush();

        return document.activeElement?.textContent;
    };
    toggle("open");
    const back = toggle("closed");
    toggle("open");
    [...container.querySelectorAll("button")].at(-1)?.focus();

    expect([back, toggle("closed")]).toEqual(["Share", "Elsewhere"]);
});

test("drop a popover's anchor and its rule once the popover unmounts", () => {
    // render a popover, read its rule, then unmount it
    const { container, unmount } = render(() => (
        <Popover>
            <PopoverTrigger>Share</PopoverTrigger>
            <PopoverContent align="start">Anyone with the link can view.</PopoverContent>
        </Popover>
    ));
    const name = container
        .querySelector("[data-slot=popover-trigger]")
        ?.getAttribute("data-anchor-name");
    const rules = () =>
        document.head.querySelectorAll(`style[data-anchor-rule="${String(name)}"]`).length;
    const mounted = rules();
    unmount();

    expect({ mounted, unmounted: rules() }).toEqual({ mounted: 1, unmounted: 0 });
});

test("open a modal popover as a modal dialog anchored by name on top of the stack, and refocus its trigger once the platform closes it", () => {
    const changes: boolean[] = [];
    const { container } = render(() => (
        <Popover modal onOpenChange={(open) => changes.push(open)}>
            <PopoverTrigger>Share</PopoverTrigger>
            <PopoverContent align="start">
                <button type="button">Copy link</button>
            </PopoverContent>
        </Popover>
    ));
    const closed = markup(container);
    const trigger = container.querySelector<HTMLElement>("[data-slot=popover-trigger]");
    const content = container.querySelector("dialog");
    trigger?.click();
    flush();
    const stack = LayerStack.of(document.body);
    const opened = [content?.open, stack.top() === content, markup(container)];

    // close the dialog as Escape or a click outside does
    content?.close();
    flush();
    expect([
        closed,
        opened,
        content?.open,
        stack.top(),
        document.activeElement === trigger,
        changes,
    ]).toEqual([
        '<button data-slot="popover-trigger" type="button" id="id-1-trigger" aria-haspopup="dialog" aria-expanded="false" aria-controls="id-1">Share</button>' +
            '<dialog id="id-1" closedby="any" aria-labelledby="id-1-trigger" data-slot="popover-content" data-closed="" data-side="bottom" data-align="start"><button type="button">Copy link</button></dialog>',
        [
            true,
            true,
            '<button data-slot="popover-trigger" type="button" id="id-1-trigger" aria-haspopup="dialog" aria-expanded="true" aria-controls="id-1" data-popup-open="" data-anchor-name="--id-1">Share</button>' +
                '<dialog id="id-1" closedby="any" aria-labelledby="id-1-trigger" data-slot="popover-content" data-side="bottom" data-align="start" data-open="" open="" data-position-anchor="--id-1"><button type="button">Copy link</button></dialog>',
        ],
        false,
        undefined,
        true,
        [true, false],
    ]);
});

test("place a modal popover beside its anchor, name it by its title, and close it from its close button", () => {
    const { container } = render(() => (
        <Popover modal>
            <PopoverAnchor>Row</PopoverAnchor>
            <PopoverTrigger>Edit</PopoverTrigger>
            <PopoverContent>
                <PopoverTitle>Rename</PopoverTitle>
                <PopoverDescription>Names show in every list.</PopoverDescription>
                <PopoverClose>Done</PopoverClose>
            </PopoverContent>
        </Popover>
    ));
    container.querySelector<HTMLElement>("[data-slot=popover-trigger]")?.click();
    flush();
    const content = container.querySelector("dialog");
    const anchored = [
        container.querySelector("[data-slot=popover-anchor]")?.getAttribute("data-anchor-name"),
        content?.getAttribute("aria-labelledby"),
        content?.getAttribute("aria-describedby"),
    ];
    container.querySelector<HTMLElement>("[data-slot=popover-close]")?.click();
    flush();

    const id = content?.id ?? "";
    expect([anchored, content?.open]).toEqual([
        [`--${id}`, `${id}-title`, `${id}-description`],
        false,
    ]);
});
