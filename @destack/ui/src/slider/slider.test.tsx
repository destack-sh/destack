import { expect, test } from "@destack/test";
import { Errored, flush } from "@destack/view";
import { Slider, SliderIndicator, SliderThumb, SliderTrack } from "./index.ts";
import { markup, render } from "@destack/view/test";

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
    const { container } = render(() => (
        <Slider min={0} max={100} step={5} defaultValue={[40]} aria-label="Volume" />
    ));
    expect(markup(container)).toBe(
        '<div role="group" data-slot="slider" data-orientation="horizontal" aria-label="Volume">' +
            '<div data-slot="slider-track" data-orientation="horizontal">' +
            '<div data-slot="slider-indicator" data-orientation="horizontal" style="--x-insetInlineStart: 0%; --x-insetInlineEnd: calc(var(--destack-space-4) / 2 + (100% - var(--destack-space-4)) * 0.6);"></div></div>' +
            '<input type="range" data-slot="slider-thumb" data-orientation="horizontal" min="0" max="100" step="5" aria-label="Volume"></div>',
    );
    expect(thumbs(container)).toEqual(["40"]);
});

test("compose a range from its parts, each thumb taking its value by its place", () => {
    const { container } = render(() => (
        <Slider defaultValue={[20, 70]} aria-label="Hours">
            <SliderTrack>
                <SliderIndicator />
            </SliderTrack>
            <SliderThumb />
            <SliderThumb />
        </Slider>
    ));
    drag(container, 1, 90);
    const range = container.querySelector("[data-slot=slider-indicator]")?.getAttribute("style");

    expect([thumbs(container), range]).toEqual([
        ["20", "90"],
        "--x-insetInlineStart: calc(var(--destack-space-4) / 2 + (100% - var(--destack-space-4)) * 0.2); --x-insetInlineEnd: calc(var(--destack-space-4) / 2 + (100% - var(--destack-space-4)) * 0.1);",
    ]);
});

test("report each value a thumb moves to, and the values it settles on", () => {
    const moved: (readonly number[])[] = [];
    const committed: (readonly number[])[] = [];
    const { container } = render(() => (
        <Slider
            min={12}
            max={24}
            defaultValue={[18]}
            aria-label="Font size"
            onValueChange={(value) => moved.push(value)}
            onValueCommitted={(value) => committed.push(value)}
        />
    ));
    drag(container, 0, 21);
    container.querySelector("input")?.dispatchEvent(new Event("change", { bubbles: true }));
    expect([moved, committed, thumbs(container)]).toEqual([[[21]], [[21]], ["21"]]);
});

test("stop a range's thumbs from crossing and name them by the end they set", () => {
    const { container } = render(() => (
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

test("leave a controlled slider at its owner's values", () => {
    const { container } = render(() => <Slider value={[50]} aria-label="Volume" />);
    drag(container, 0, 80);
    expect(thumbs(container)).toEqual(["50"]);
});

test("refuse a thumb the slider's values have none for", () => {
    const { container } = render(() => (
        <Errored fallback={(error) => String(error())}>
            <Slider defaultValue={[20]} aria-label="Volume">
                <SliderThumb />
                <SliderThumb />
            </Slider>
        </Errored>
    ));

    expect(container.textContent).toBe("RangeError: a slider has no value for thumb '1'");
});

test("label a single thumb as its slider, by name or by the element it points to", () => {
    // one slider named by an attribute and one by its heading
    const { container } = render(() => (
        <>
            <Slider defaultValue={[40]} aria-label="Volume" />
            <h2 id="brightness">Brightness</h2>
            <Slider defaultValue={[60]} aria-labelledby="brightness" />
        </>
    ));

    expect(
        [...container.querySelectorAll("[data-slot=slider-thumb]")].map((thumb) => [
            thumb.getAttribute("aria-label"),
            thumb.getAttribute("aria-labelledby"),
        ]),
    ).toEqual([
        ["Volume", null],
        [null, "brightness"],
    ]);
});
