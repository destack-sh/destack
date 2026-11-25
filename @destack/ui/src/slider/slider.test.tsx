import { expect, test } from "@destack/test";
import { Errored, flush } from "@destack/view";
import { Slider, SliderIndicator, SliderThumb, SliderTrack, SliderValue } from "./index.ts";
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
        '<div data-slot="slider" role="group" data-orientation="horizontal" aria-label="Volume">' +
            '<div data-slot="slider-track" data-orientation="horizontal">' +
            '<div data-slot="slider-indicator" style="--x-insetInlineStart: 0%; --x-insetInlineEnd: calc(var(--destack-space-4) / 2 + (100% - var(--destack-space-4)) * 0.6);" data-orientation="horizontal"></div></div>' +
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

/** Press a key on a container's thumb. */
function press(container: Element, index: number, key: string, shiftKey = false): void {
    const thumb = container.querySelectorAll("input")[index];
    thumb?.dispatchEvent(new KeyboardEvent("keydown", { key, shiftKey, bubbles: true }));
    flush();
}

test("take large steps by the keys, keep the least steps between, and write the values", () => {
    const committed: (readonly number[])[] = [];
    const { container } = render(() => (
        <Slider
            max={500}
            step={10}
            largeStep={5}
            minStepsBetweenValues={2}
            defaultValue={[100, 300]}
            format={{ style: "currency", currency: "EUR", maximumFractionDigits: 0 }}
            onValueCommitted={(values) => committed.push(values)}
        >
            <SliderValue />
            <SliderThumb />
            <SliderThumb />
        </Slider>
    ));
    press(container, 0, "PageUp");
    press(container, 0, "ArrowUp", true);
    press(container, 1, "PageDown");
    drag(container, 0, 400);

    expect([
        thumbs(container),
        committed,
        container.querySelector("output")?.textContent,
        container.querySelector("input")?.getAttribute("aria-valuetext"),
    ]).toEqual([
        ["230", "250"],
        [
            [150, 300],
            [200, 300],
            [200, 250],
        ],
        "€230 – €250",
        "€230",
    ]);
});

/** Send a pointer over a slider 120 wide, whose thumbs travel 100. */
function point(container: Element, type: string, x: number): void {
    const slider = container.querySelector<HTMLElement>("[data-slot=slider]");
    if (slider !== null) {
        slider.getBoundingClientRect = () => DOMRect.fromRect({ width: 120, height: 20 });
        slider.setPointerCapture = () => {};
        slider.dispatchEvent(
            new PointerEvent(type, { clientX: x, clientY: 10, pointerId: 1, bubbles: true }),
        );
    }
    flush();
}

test("drag the thumb nearest a press anywhere on the track, and put it back when cancelled", () => {
    const committed: (readonly number[])[] = [];
    const { container } = render(() => (
        <Slider defaultValue={[20, 80]} onValueCommitted={(values) => committed.push(values)} />
    ));
    const places: string[][] = [];
    const step = (type: string, x: number) => {
        point(container, type, x);
        places.push(thumbs(container));
    };
    step("pointerdown", 40);
    step("pointermove", 60);
    step("pointerup", 60);
    step("pointerdown", 110);
    step("pointermove", 10);
    step("pointercancel", 10);

    expect([places, committed]).toEqual([
        [
            ["30", "80"],
            ["50", "80"],
            ["50", "80"],
            ["50", "100"],
            ["50", "50"],
            ["50", "80"],
        ],
        [[50, 80]],
    ]);
});

test("let a person read a slider they cannot move", () => {
    const { container } = render(() => <Slider defaultValue={[20]} readOnly />);
    drag(container, 0, 60);
    press(container, 0, "PageUp");
    point(container, "pointerdown", 110);

    expect([
        thumbs(container),
        container.querySelector("input")?.getAttribute("aria-readonly"),
    ]).toEqual([["20"], "true"]);
});

test("step by decimals without floating point's remainder", () => {
    const { container } = render(() => <Slider max={1} step={0.1} defaultValue={[0.2]} />);
    press(container, 0, "PageUp");
    drag(container, 0, 0.3);

    expect(thumbs(container)).toEqual(["0.3"]);
});
