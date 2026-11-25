import { expect, test } from "@destack/test";
import { classes, markup, render } from "@destack/view/test";
import { Message } from "../message/index.ts";
import { Bubble, BubbleContent, BubbleGroup, BubbleOverlay } from "./index.ts";

test("render a bubble with its look and side, and a button of one", () => {
    const { container } = render(() => (
        <>
            <Bubble variant="tinted" align="end">
                <BubbleContent render={(attributes) => <button {...attributes} />}>
                    Yes
                </BubbleContent>
                <BubbleOverlay side="top">👍</BubbleOverlay>
            </Bubble>
        </>
    ));

    expect(markup(container)).toBe(
        [
            '<div data-slot="bubble" data-variant="tinted" data-align="end">',
            '<button data-slot="bubble-content">Yes</button>',
            '<div data-slot="bubble-overlay" data-side="top" data-align="end">👍</div></div>',
        ].join(""),
    );
});

test("stand at the side of the message around it, unless it has its own", () => {
    const { container } = render(() => (
        <Message align="end">
            <Bubble />
            <Bubble align="start" />
        </Message>
    ));

    expect(
        [...container.querySelectorAll("[data-slot=bubble]")].map((bubble) =>
            bubble.getAttribute("data-align"),
        ),
    ).toEqual(["end", "start"]);
});

test("round a bubble alone, and tighten the corners of one in a group", () => {
    const { container } = render(() => (
        <>
            <Bubble>
                <BubbleContent>Alone</BubbleContent>
            </Bubble>
            <BubbleGroup>
                <Bubble>
                    <BubbleContent>First</BubbleContent>
                </Bubble>
            </BubbleGroup>
        </>
    ));
    const [alone, first] = [...container.querySelectorAll("[data-slot=bubble]")].map(
        (bubble) => classes(bubble)[0]?.split(" ").length ?? 0,
    );

    expect((first ?? 0) - (alone ?? 0)).toBe(2);
});
