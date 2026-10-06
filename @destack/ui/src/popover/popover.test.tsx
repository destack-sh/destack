import { expect, test } from "@destack/test";
import { Popover, PopoverContent, PopoverTrigger } from "./index.ts";
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
            '<div id="id-1" popover="auto" role="dialog" aria-labelledby="id-1-trigger" data-slot="popover-content" data-side="bottom" data-align="start">Anyone with the link can view.</div>',
    );
});
