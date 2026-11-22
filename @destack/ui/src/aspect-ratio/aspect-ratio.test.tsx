import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { AspectRatio } from "./index.ts";

test("size a box by its width-to-height ratio, square by default", () => {
    const { container } = render(() => (
        <>
            <AspectRatio ratio={16 / 9} />
            <AspectRatio />
        </>
    ));
    const boxes = [...container.querySelectorAll("[data-slot=aspect-ratio]")];

    // each box marks its ratio for its styles to read
    expect(boxes.map((box) => box.getAttribute("style")?.split(": ")[1])).toEqual([
        `${16 / 9};`,
        "1;",
    ]);
});
