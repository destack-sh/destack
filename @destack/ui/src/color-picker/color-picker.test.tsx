import { ColorSwatchPicker } from "../color-swatch-picker/index.ts";
import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { markup, render } from "@destack/view/test";
import { ColorArea, ColorField, ColorPicker, ColorSlider, ColorSwatch } from "./index.ts";

/** Find an element of a container. */
function find(container: Element, selector: string): HTMLInputElement {
    const element = container.querySelector<HTMLInputElement>(selector);
    if (element === null) {
        throw new TypeError(`the test picker has no '${selector}'`);
    }

    return element;
}

/** Press keys on an element. */
function press(element: Element, ...keys: readonly (string | KeyboardEventInit)[]): void {
    for (const key of keys) {
        const given = typeof key === "string" ? { key } : key;
        element.dispatchEvent(
            new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...given }),
        );
        flush();
    }
}

test("render a color's area, hue and text, and submit it by its name", () => {
    const { container } = render(() => (
        <ColorPicker name="accent" defaultValue="#f76b15" aria-label="Accent">
            <ColorArea />
            <ColorSwatch />
            <ColorField />
        </ColorPicker>
    ));

    expect(markup(container)).toBe(
        [
            '<div data-slot="color-picker" role="group" aria-label="Accent">',
            '<div data-slot="color-area" role="group">',
            '<canvas width="48" height="48" aria-hidden="true" data-slot="color-area-paint"></canvas>',
            '<div data-slot="color-thumb" style="--x-insetInlineStart: 91.49797570850203%; --x-insetBlockEnd: 96.86274509803921%; --x-backgroundColor: #f76b15;">',
            '<input type="range" aria-label="Color" aria-roledescription="2D slider" aria-valuetext="Saturation 91, Brightness 97, \u2068vibrant\u2069 \u2068\u2068red\u2069-\u2068orange\u2069\u2069" min="0" max="100" step="1"></div></div>',
            '<span data-slot="color-swatch" role="img" aria-label="\u2068vibrant\u2069 \u2068\u2068red\u2069-\u2068orange\u2069\u2069"><span style="--x-backgroundColor: #f76b15;"></span></span>',
            '<input data-slot="color-field" aria-label="Color" dir="ltr" autocomplete="off">',
            '<input type="hidden" name="accent" value="#f76b15"></div>',
        ].join(""),
    );
});

test("step an area by the keys, a tenth with Shift, and keep the hue of a gray", () => {
    const given: string[] = [];
    const committed: string[] = [];
    const { container } = render(() => (
        <ColorPicker
            defaultValue="#30a46c"
            onValueChange={(value) => void given.push(value)}
            onValueCommitted={(value) => void committed.push(value)}
        >
            <ColorArea />
        </ColorPicker>
    ));
    const input = find(container, "[data-slot=color-area] input");
    press(input, "ArrowRight", "ArrowUp", { key: "ArrowLeft", shiftKey: true }, "PageDown", "End");
    for (let count = 0; count < 12; count += 1) {
        press(input, "Home");
    }
    const gray = given.at(-1);
    press(input, "End", "End");

    expect({
        given: given.slice(0, 5),
        gray,
        back: given.at(-1),
        isCommitted: committed.join() === given.join(),
    }).toEqual({
        given: ["#2ea46b", "#2fa76d", "#40a775", "#368d63", "#288d5c"],
        gray: "#8d8d8d",
        back: "#718d7f",
        isCommitted: true,
    });
});

test("set a channel by a slider and a number, and the whole color by text and a swatch", () => {
    const [value, setValue] = createSignal("rgb(10 20 30 / 0.5)");
    const { container } = render(() => (
        <ColorPicker value={value()} format="rgb" onValueChange={(next) => void setValue(next)}>
            <ColorSlider channel="alpha" />
            <ColorField channel="red" colorSpace="rgb" />
            <ColorField />
            <ColorSwatchPicker colors={["#30a46c"]} />
        </ColorPicker>
    ));
    const read: string[] = [];
    const settle = (): void => {
        flush();
        read.push(value());
    };

    // slide the opacity
    const slider = find(container, "[data-slot=slider-thumb]");
    slider.value = "0.25";
    slider.dispatchEvent(new Event("input", { bubbles: true }));
    settle();

    // type a color with and without its mark, and what is none
    const field = find(container, "input[data-slot=color-field]");
    for (const typed of ["f80", "hsl(200 50% 50%)", "nothing"]) {
        field.value = typed;
        press(field, "Enter");
        settle();
    }
    const left = field.value;

    // choose a swatch
    const swatch = find(container, "[data-slot=color-swatch-picker-item]");
    const before = swatch.getAttribute("aria-selected");
    swatch.click();
    settle();

    expect({ read, left, selected: [before, swatch.getAttribute("aria-selected")] }).toEqual({
        read: [
            "rgb(10 20 30 / 0.25)",
            "rgb(255 136 0)",
            "rgb(64 149 191)",
            "rgb(64 149 191)",
            "rgb(48 164 108)",
        ],
        left: "rgb(64 149 191)",
        selected: ["false", "true"],
    });
});

test("leave a disabled picker as it is", () => {
    const given: string[] = [];
    const { container } = render(() => (
        <ColorPicker disabled onValueChange={(value) => void given.push(value)}>
            <ColorArea />
            <ColorSwatchPicker colors={["#30a46c"]} />
            <ColorField />
        </ColorPicker>
    ));
    press(find(container, "[data-slot=color-area] input"), "ArrowUp");
    find(container, "[data-slot=color-swatch-picker-item]").click();
    flush();

    expect({
        given,
        disabled: [...container.querySelectorAll("input, button, [role=option]")].map((element) =>
            element.matches(":disabled, [aria-disabled=true]"),
        ),
    }).toEqual({ given: [], disabled: [true, true, true] });
});

test("name a swatch by how light and strong it is, its hue and how far it is see-through", () => {
    const { container } = render(() => (
        <>
            <ColorSwatch color="#000080" />
            <ColorSwatch color="#add8e6" />
            <ColorSwatch color="#ff000080" />
        </>
    ));

    expect(
        [...container.querySelectorAll("[data-slot=color-swatch]")].map((swatch) =>
            swatch.getAttribute("aria-label")?.replaceAll(/[\u2068\u2069]/gu, ""),
        ),
    ).toEqual([
        "very dark vibrant blue",
        "very light pale cyan-blue",
        "vibrant red, 50% transparent",
    ]);
});
