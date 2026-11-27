import { BoxSides, Corners, Dimension, Layout, ShadowStack, Stroke } from "@destack/style";
import { JsonValue, present, schema, toJsonSchema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import { ValueField } from "../value-field/index.ts";

/** Draw a design value's input from a value, recording what it gives. */
function draw(definition: schema.Schema, start: JsonValue | undefined) {
    stubPopovers();
    const given: (JsonValue | undefined)[] = [];
    const [value, setValue] = createSignal(start);
    const { container } = render(() => (
        <ValueField
            type={{ kind: "json", schema: JsonValue.object(toJsonSchema(definition)) }}
            value={value()}
            onValueChange={(next) => {
                given.push(next);
                setValue(() => next);
            }}
        />
    ));

    return { container, given };
}

/** Type a number into the number field of a name. */
function type(root: Element, label: string, text: string): void {
    const group = present(root.querySelector(`[aria-label="${label}"]`), `the field ${label}`);
    const input = present(
        group instanceof HTMLInputElement ? group : group.querySelector("input"),
        `the input ${label}`,
    );
    input.value = text;
    input.dispatchEvent(new InputEvent("input", { bubbles: true }));
    flush();
}

/** Press the button of a name. */
function press(root: Element, label: string, nth = 0): void {
    const buttons = [...root.querySelectorAll<HTMLButtonElement>("button")].filter(
        (each) => (each.getAttribute("aria-label") ?? each.textContent) === label,
    );
    present(buttons[nth], `the button ${label}`).click();
    flush();
}

test("give a length by its number, kept in its unit", () => {
    const { container, given } = draw(Dimension, "12px");
    type(container, "Amount", "16");

    expect(given).toEqual(["16px"]);
});

test("round every corner by one radius, then one corner on its own", () => {
    // set all four, then turn each one apart and change the top left
    const { container, given } = draw(Corners, Corners.all("8px"));
    type(
        present(container.querySelector("[role=group] [role=group]"), "the radius"),
        "Amount",
        "12",
    );
    press(container, "Each corner");
    type(container, "Top left", "0");

    expect(given).toEqual([Corners.all("12px"), { ...Corners.all("12px"), topLeft: "0px" }]);
});

test("draw a stroke outside the box", () => {
    const start: Stroke = { color: "#000000", width: "1px", align: "inside", style: "solid" };
    const { container, given } = draw(Stroke, start);
    press(container, "Outside");

    expect(given).toEqual([{ ...start, align: "outside" }]);
});

test("add a shadow on top, set it inside, and remove the one below", () => {
    // start from one shadow, add another, turn the new one inward, then remove the old one
    const old = { color: "#00000033", x: "0", y: "1px", blur: "2px", spread: "0", isInset: false };
    const { container, given } = draw(ShadowStack, [old]);
    press(container, "Add shadow");
    present(
        container.querySelector<HTMLElement>("[role=switch]"),
        "the first shadow's switch",
    ).click();
    flush();
    press(container, "Remove shadow", 1);
    const added = {
        color: "#00000033",
        x: "0",
        y: "4px",
        blur: "12px",
        spread: "0",
        isInset: false,
    };

    expect(given).toEqual([
        [added, old],
        [{ ...added, isInset: true }, old],
        [{ ...added, isInset: true }],
    ]);
});

test("lay children out in a column, with the padding its sides input gives", () => {
    // turn the layout into a column, then pad every side
    const start: Layout = {
        direction: "row",
        gap: "8px",
        align: "start",
        justify: "start",
        isWrapping: false,
        padding: BoxSides.all("0"),
    };
    const { container, given } = draw(Layout, start);
    press(container, "Column");
    type(present(container.querySelector("[aria-label=Padding]"), "the padding"), "Amount", "16");

    expect(given).toEqual([
        { ...start, direction: "column" },
        { ...start, direction: "column", padding: BoxSides.all("16px") },
    ]);
});
