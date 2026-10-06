import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { Slider } from "./index.ts";
import { draw, markup } from "@destack/view/test";

/** Move a thumb of a container to a value as the person drags it. */
function drag(container: Element, index: number, value: number): void {
    const thumb = container.querySelectorAll("input")[index];
    if (thumb !== undefined) {
        thumb.value = String(value);
        thumb.dispatchEvent(new Event("input", { bubbles: true }));
    }
    flush();
}

/** List the values of a container's thumbs, in order. */
function thumbs(container: Element): string[] {
    return [...container.querySelectorAll("input")].map((input) => input.value);
}

test("render a native range input per thumb over a track filled up to the value", () => {
    const container = draw(() => (
        <Slider min={0} max={100} step={5} defaultValue={[40]} aria-label="Volume" />
    ));
    expect(markup(container)).toBe(
        '<div role="group" data-slot="slider" data-orientation="horizontal" aria-label="Volume">' +
            '<div data-slot="slider-track" style="--x-background-image: linear-gradient(to right, var(--destack-color-muted) 0%, var(--destack-color-primary) 0%, var(--destack-color-primary) 40%, var(--destack-color-muted) 40%);"></div>' +
            '<input type="range" data-slot="slider-thumb" min="0" max="100" step="5"></div>',
    );
    expect(thumbs(container)).toEqual(["40"]);
});

test("report each value a thumb moves to, and the values it settles on", () => {
    const moved: (readonly number[])[] = [];
    const committed: (readonly number[])[] = [];
    const container = draw(() => (
        <Slider
            min={12}
            max={24}
            defaultValue={[18]}
            aria-label="Font size"
            onValueChange={(value) => moved.push(value)}
            onValueCommit={(value) => committed.push(value)}
        />
    ));
    drag(container, 0, 21);
    container.querySelector("input")?.dispatchEvent(new Event("change", { bubbles: true }));
    expect([moved, committed, thumbs(container)]).toEqual([[[21]], [[21]], ["21"]]);
});

test("keep a range's thumbs from crossing and name them by the end they set", () => {
    const container = draw(() => (
        <Slider min={0} max={500} defaultValue={[100, 300]} aria-label="Price" />
    ));
    drag(container, 0, 400);
    const names = [...container.querySelectorAll("input")].map((input) =>
        input.getAttribute("aria-label"),
    );
    expect([thumbs(container), names]).toEqual([
        ["300", "300"],
        ["Minimum", "Maximum"],
    ]);
});

test("hold a controlled slider at its owner's values", () => {
    const container = draw(() => <Slider value={[50]} aria-label="Volume" />);
    drag(container, 0, 80);
    expect(thumbs(container)).toEqual(["50"]);
});
