import { type Fill, FillStack, type Paint } from "@destack/style";
import { JsonValue, present, toJsonSchema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import { ValueField } from "../value-field/index.ts";
import { PaintPicker } from "./index.ts";

/** Draw a paint picker from a paint, recording what it gives. */
function draw(start: Paint) {
    const given: Paint[] = [];
    const { container } = render(() => (
        <PaintPicker defaultValue={start} onValueChange={(next) => given.push(next)} />
    ));

    return { container, given };
}

/** Press the button whose text or name is a label. */
function press(root: Element, label: string): void {
    const buttons = [...root.querySelectorAll<HTMLButtonElement>("button")];
    present(
        buttons.find((each) => (each.getAttribute("aria-label") ?? each.textContent) === label),
        `the button ${label}`,
    ).click();
    flush();
}

/** Press a key on an element. */
function key(element: Element, name: string, shiftKey = false): void {
    element.dispatchEvent(new KeyboardEvent("keydown", { key: name, shiftKey, bubbles: true }));
    flush();
}

test("turn a color into a gradient fading it out, and back into its first stop", () => {
    // switch a blue to a gradient, then back to one color
    const { container, given } = draw({ kind: "solid", color: "#0000ff" });
    press(container, "Gradient");
    press(container, "Solid");

    expect(given).toEqual([
        {
            kind: "gradient",
            gradient: {
                shape: { kind: "linear", angle: 90 },
                stops: [
                    { color: "#0000ff", position: 0 },
                    { color: "#0000ff00", position: 1 },
                ],
                space: "oklab",
                hue: "shorter",
            },
        },
        { kind: "solid", color: "#0000ff" },
    ]);
});

test("move a stop by its keys, remove one, and spread the stops round a center", () => {
    // nudge the first of three stops a tenth along, remove the second, then make it conic
    const { container, given } = draw({
        kind: "gradient",
        gradient: {
            shape: { kind: "linear", angle: 45 },
            stops: [
                { color: "#241d9a", position: 0 },
                { color: "#f75092", position: 0.5 },
                { color: "#ffc53d", position: 1 },
            ],
            space: "oklab",
            hue: "shorter",
        },
    });
    const stops = () => [...container.querySelectorAll("[data-slot=gradient-picker-stop]")];
    key(present(stops()[0], "the first stop"), "ArrowRight", true);
    key(present(stops()[1], "the second stop"), "Delete");
    press(container, "Conic");

    // expect two stops left, the first a tenth along, round the center from the line's angle
    expect({ count: stops().length, last: given.at(-1) }).toEqual({
        count: 2,
        last: {
            kind: "gradient",
            gradient: {
                shape: { kind: "conic", center: { x: 0.5, y: 0.5 }, angle: 45 },
                stops: [
                    { color: "#241d9a", position: 0.1 },
                    { color: "#ffc53d", position: 1 },
                ],
                space: "oklab",
                hue: "shorter",
            },
        },
    });
});

test("give an image once it has an address, covering the box from its center", () => {
    // show the image kind, then type an address
    const { container, given } = draw({ kind: "solid", color: "#000000" });
    press(container, "Image");
    const address = present(
        container.querySelector<HTMLInputElement>("input[type=url]"),
        "the address",
    );
    address.value = " https://images.example.com/paper.png ";
    address.dispatchEvent(new Event("change", { bubbles: true }));
    flush();

    expect(given).toEqual([
        {
            kind: "image",
            source: "https://images.example.com/paper.png",
            fit: "cover",
            position: { x: 0.5, y: 0.5 },
        },
    ]);
});

test("add a layer on top, hide one and remove one, in a fill stack's input", () => {
    // draw a stack of one white layer through its named input
    stubPopovers();
    const given: (JsonValue | undefined)[] = [];
    const white: Fill = {
        paint: { kind: "solid", color: "#ffffff" },
        blend: "normal",
        isVisible: true,
    };
    const [value, setValue] = createSignal<JsonValue | undefined>([white]);
    const { container } = render(() => (
        <ValueField
            type={{ kind: "json", schema: JsonValue.object(toJsonSchema(FillStack)) }}
            value={value()}
            onValueChange={(next) => {
                given.push(next);
                setValue(() => next);
            }}
        />
    ));

    // add a layer, hide the white one below it, then remove the new one
    press(container, "Add layer");
    const rows = () => [...container.querySelectorAll("[data-slot=sortable-item]")];
    const whiteRow = present(rows()[1], "the white layer");
    const toggles = [...container.querySelectorAll<HTMLButtonElement>("[aria-label='Show layer']")];
    present(toggles[1], "the white layer's toggle").click();
    flush();
    const isKept = rows()[1] === whiteRow;
    const removes = [
        ...container.querySelectorAll<HTMLButtonElement>("[aria-label='Remove layer']"),
    ];
    present(removes[0], "the new layer's remove").click();
    flush();

    const black: Fill = {
        paint: { kind: "solid", color: "#000000" },
        blend: "normal",
        isVisible: true,
    };
    // expect each change, the hidden layer's row kept as it changed
    expect({ given, isKept }).toEqual({
        given: [
            [black, white],
            [black, { ...white, isVisible: false }],
            [{ ...white, isVisible: false }],
        ],
        isKept: true,
    });
});
