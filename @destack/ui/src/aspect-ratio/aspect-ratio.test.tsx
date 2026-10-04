import { expect, test } from "@destack/test";
import { draw } from "@destack/view/test";
import { AspectRatio } from "./index.ts";

test("keep a box's width-to-height ratio, square by default", () => {
    const container = draw(() => (
        <>
            <AspectRatio ratio={16 / 9} />
            <AspectRatio />
        </>
    ));
    const boxes = [...container.querySelectorAll("[data-slot=aspect-ratio]")];

    // each box carries its ratio for its styles to read
    expect(boxes.map((box) => box.getAttribute("style")?.split(": ")[1])).toEqual([
        `${16 / 9};`,
        "1;",
    ]);
});
