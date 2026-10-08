import { expect, test } from "@destack/test";
import { Errored, flush } from "@destack/view";
import { Slider, SliderRange, SliderThumb, SliderTrack } from "./index.ts";
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

test("render a native range input per thumb over a track whose range fills up to the value", () => {
    const container = draw(() => (
        <Slider min={0} max={100} step={5} defaultValue={[40]} aria-label="Volume" />
    ));
    expect(markup(container)).toBe(
        '<div role="group" data-slot="slider" data-orientation="horizontal" aria-label="Volume">' +
            '<div data-slot="slider-track" data-orientation="horizontal">' +
            '<div data-slot="slider-range" data-orientation="horizontal" style="--x-insetInlineStart: 0%; --x-insetInlineEnd: 60%;"></div></div>' +
            '<input type="range" data-slot="slider-thumb" data-orientation="horizontal" min="0" max="100" step="5"></div>',
    );
    expect(thumbs(container)).toEqual(["40"]);
});

test("compose a range from its parts, each thumb taking its value by its place", () => {
    const container = draw(() => (
        <Slider defaultValue={[20, 70]} aria-label="Hours">
            <SliderTrack>
                <SliderRange />
            </SliderTrack>
            <SliderThumb />
            <SliderThumb />
        </Slider>
    ));
    drag(container, 1, 90);
    const range = container.querySelector("[data-slot=slider-range]")?.getAttribute("style");

    expect([thumbs(container), range]).toEqual([
        ["20", "90"],
        "--x-insetInlineStart: 20%; --x-insetInlineEnd: 10%;",
    ]);
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

test("refuse a thumb the slider's values have none for", () => {
    const container = draw(() => (
        <Errored fallback={(error) => String(error())}>
            <Slider defaultValue={[20]} aria-label="Volume">
                <SliderThumb />
                <SliderThumb />
            </Slider>
        </Errored>
    ));

    expect(container.textContent).toBe("RangeError: a slider has no value for thumb 1");
});
