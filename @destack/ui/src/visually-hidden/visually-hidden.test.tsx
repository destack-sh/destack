import { expect, test } from "@destack/test";
import { draw, markup } from "@destack/view/test";
import { visuallyHiddenIconButton } from "./visually-hidden.example.tsx";
import { VisuallyHidden } from "./index.ts";

test("render content in a span that keeps the element's own attributes", () => {
    const container = draw(() => <VisuallyHidden id="name">Delete note</VisuallyHidden>);

    expect(markup(container)).toBe(
        '<span data-slot="visually-hidden" id="name">Delete note</span>',
    );
});

test("name an icon button by its visually hidden text", () => {
    const container = draw(visuallyHiddenIconButton);

    expect(container.querySelector("button")?.textContent).toBe("Delete note");
});
