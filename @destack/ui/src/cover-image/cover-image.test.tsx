import { expect, test } from "@destack/test";
import { draw } from "@destack/view/test";
import { flush } from "@destack/view";
import { CoverImage } from "./index.ts";

/** Press a key on a container's repositioning slider. */
function press(container: Element, key: string): void {
    container
        .querySelector("input[type=range]")
        ?.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
    flush();
}

test("centre the banner on the position, and move the position a step per arrow key within the image", () => {
    const moved: number[] = [];
    const container = draw(() => (
        <CoverImage
            source="/covers/roadmap.webp"
            position={0.98}
            label="Reposition the cover"
            onPositionChange={(position) => moved.push(position)}
        />
    ));
    const handle = container.querySelector<HTMLInputElement>("input[type=range]");
    press(container, "ArrowDown");
    press(container, "ArrowUp");

    expect({
        value: handle?.value,
        moved: moved.map((position) => Math.round(position * 100) / 100),
        still: draw(() => (
            <CoverImage source="/covers/roadmap.webp" position={0.5} />
        )).querySelector("input[type=range]"),
    }).toEqual({ value: "98", moved: [1, 0.93], still: null });
});
