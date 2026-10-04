import { expect, test } from "@destack/test";
import { draw, markup } from "@destack/view/test";
import { Bubble, BubbleContent, BubbleReactions } from "./index.ts";

test("render a bubble with its variant, alignment and reactions", () => {
    const container = draw(() => (
        <Bubble variant="tinted" align="end">
            <BubbleContent>Booked</BubbleContent>
            <BubbleReactions>🎉</BubbleReactions>
        </Bubble>
    ));
    expect(markup(container)).toBe(
        '<div data-slot="bubble" data-variant="tinted" data-align="end">' +
            '<div data-slot="bubble-content">Booked</div>' +
            '<div data-slot="bubble-reactions" data-side="bottom" data-align="end">🎉</div></div>',
    );
});

test("color a bubble's content by the bubble's variant", () => {
    const container = draw(() => (
        <>
            <Bubble>
                <BubbleContent>Default</BubbleContent>
            </Bubble>
            <Bubble variant="outline">
                <BubbleContent>Outline</BubbleContent>
            </Bubble>
        </>
    ));
    const [first, second] = [...container.querySelectorAll("[data-slot=bubble-content]")];
    expect(first?.className).not.toBe(second?.className);
});
