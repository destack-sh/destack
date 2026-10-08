import { expect, test } from "@destack/test";
import { ScrollArea } from "./index.ts";
import { classes, render } from "@destack/view/test";

test("render a focusable scroll area that scrolls along its orientation's axes", () => {
    const { container } = render(() => (
        <>
            <ScrollArea aria-label="Tags">travel</ScrollArea>
            <ScrollArea orientation="horizontal" />
            <ScrollArea orientation="both" />
        </>
    ));

    // keyboard users reach the region to scroll it, and each orientation scrolls differently
    expect(container.firstElementChild?.outerHTML.replace(/ class="[^"]*"/u, "")).toBe(
        '<div tabindex="0" data-slot="scroll-area" data-orientation="vertical" aria-label="Tags">travel</div>',
    );
    expect(new Set(classes(container)).size).toBe(3);
});
