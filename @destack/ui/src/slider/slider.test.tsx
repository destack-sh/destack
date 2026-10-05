import { expect, test } from "@destack/test";
import { flush } from "solid-js";
import { Slider } from "./index.ts";
import { draw, markup } from "@destack/view/test";

test("render a native range input filled up to its value", () => {
    const container = draw(() => (
        <Slider min={0} max={100} step={5} value={40} aria-label="Volume" />
    ));
    expect(markup(container)).toBe(
        '<input type="range" data-slot="slider" min="0" max="100" step="5" aria-label="Volume" style="--destack-slider-fill: 40%;">',
    );
    expect(container.querySelector("input")?.value).toBe("40");
});

test("fill a slider's track as far as the value moves through its range", () => {
    const values: string[] = [];
    const container = draw(() => (
        <Slider
            min={12}
            max={24}
            value={18}
            aria-label="Font size"
            onInput={(event) => values.push(event.currentTarget.value)}
        />
    ));
    const slider = container.querySelector("input");
    const fills = [slider?.style.getPropertyValue("--destack-slider-fill")];
    if (slider !== null) {
        slider.value = "21";
        slider.dispatchEvent(new Event("input", { bubbles: true }));
    }
    flush();
    fills.push(slider?.style.getPropertyValue("--destack-slider-fill"));

    // move halfway and to three quarters, telling the caller's handler the new value
    expect([fills, values]).toEqual([["50%", "75%"], ["21"]]);
});
