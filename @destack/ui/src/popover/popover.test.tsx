import { expect, test } from "@destack/test";
import { Popover, PopoverContent, PopoverTrigger } from "./index.ts";
import { flush } from "@destack/view";
import { draw, markup } from "@destack/view/test";

test("toggle a popover from its trigger through the platform's popover target, named by the trigger", () => {
    const container = draw(() => (
        <Popover>
            <PopoverTrigger variant="outline">Share</PopoverTrigger>
            <PopoverContent align="start">Anyone with the link can view.</PopoverContent>
        </Popover>
    ));
    expect(markup(container)).toBe(
        '<button data-slot="popover-trigger" data-variant="outline" data-size="default" id="id-1-trigger" popovertarget="id-1" aria-haspopup="dialog" aria-controls="id-1">Share</button>' +
            '<div id="id-1" popover="auto" role="dialog" aria-labelledby="id-1-trigger" data-slot="popover-content" data-state="closed" data-side="bottom" data-align="start">Anyone with the link can view.</div>',
    );
});

test("open a modal popover as a modal dialog anchored by name that locks the page's scroll, and refocus its trigger once the platform closes it", () => {
    const changes: boolean[] = [];
    const container = draw(() => (
        <Popover modal onOpenChange={(open) => changes.push(open)}>
            <PopoverTrigger variant="outline">Share</PopoverTrigger>
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
    const opened = [content?.open, document.documentElement.style.overflow, markup(container)];

    // close the dialog as Escape or a click outside does
    content?.close();
    flush();
    expect([
        closed,
        opened,
        content?.open,
        document.documentElement.style.overflow,
        document.activeElement === trigger,
        changes,
    ]).toEqual([
        '<button data-slot="popover-trigger" data-variant="outline" data-size="default" id="id-1-trigger" aria-haspopup="dialog" aria-expanded="false" aria-controls="id-1">Share</button>' +
            '<dialog id="id-1" closedby="any" aria-labelledby="id-1-trigger" data-slot="popover-content" data-state="closed" data-side="bottom" data-align="start"><button type="button">Copy link</button></dialog>',
        [
            true,
            "hidden",
            '<button data-slot="popover-trigger" data-variant="outline" data-size="default" id="id-1-trigger" aria-haspopup="dialog" aria-expanded="true" aria-controls="id-1" style="anchor-name: --id-1;">Share</button>' +
                '<dialog id="id-1" closedby="any" aria-labelledby="id-1-trigger" data-slot="popover-content" data-state="open" data-side="bottom" data-align="start" open="" style="position-anchor: --id-1;"><button type="button">Copy link</button></dialog>',
        ],
        false,
        "",
        true,
        [true, false],
    ]);
});
