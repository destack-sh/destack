import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { markup, render } from "@destack/view/test";
import { layOutElements } from "../virtualizer/test/layout.ts";
import { ScrollArea, ScrollAreaBar } from "./index.ts";

test("render a focusable scroll area that scrolls along its orientation's axes", () => {
    const { container } = render(() => (
        <>
            <ScrollArea aria-label="Tags">travel</ScrollArea>
            <ScrollArea orientation="horizontal" />
            <ScrollArea orientation="both" />
        </>
    ));

    // keyboard users reach the region to scroll it, and each orientation scrolls differently
    expect(markup(render(() => <ScrollArea aria-label="Tags">travel</ScrollArea>).container)).toBe(
        '<div data-slot="scroll-area" data-orientation="vertical">' +
            '<div data-slot="scroll-area-viewport" tabindex="0" aria-label="Tags">travel</div>' +
            '<div data-slot="scroll-area-bar" aria-hidden="true" tabindex="-1" data-orientation="vertical" hidden=""><div data-slot="scroll-area-thumb" style="top: 0px;"></div></div></div>',
    );
    const viewports = [...container.querySelectorAll("[data-slot=scroll-area-viewport]")];
    expect(new Set(viewports.map((viewport) => viewport.className)).size).toBe(3);
});

test("show a bar where more is past the view, its thumb as long as the share in view", () => {
    layOutElements({ client: 100, scroll: 400 });
    const { container } = render(() => <ScrollArea>long</ScrollArea>);
    flush();
    const bar = container.querySelector<HTMLElement>("[data-slot=scroll-area-bar]");

    expect({
        hidden: bar?.hidden,
        length: container.querySelector<HTMLElement>("[data-slot=scroll-area-thumb]")?.style.height,
        sides: container
            .querySelector("[data-slot=scroll-area-viewport]")
            ?.getAttribute("data-scrollable"),
    }).toEqual({ hidden: false, length: "25px", sides: "bottom right" });
});

test("leave out the bars, or draw one's own", () => {
    const { container } = render(() => (
        <>
            <ScrollArea bars={null}>one</ScrollArea>
            <ScrollArea
                bars={
                    <ScrollAreaBar>
                        <b>thumb</b>
                    </ScrollAreaBar>
                }
            >
                two
            </ScrollArea>
        </>
    ));

    expect([
        container.querySelectorAll("[data-slot=scroll-area-bar]").length,
        container.querySelector("[data-slot=scroll-area-bar]")?.textContent,
    ]).toEqual([1, "thumb"]);
});
