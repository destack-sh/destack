import { Color, Palette } from "@destack/color";
import type { ActionValue, Choice } from "@destack/object";
import { type JsonObject, JsonValue, present, schema, toJsonSchema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import { ValueField, ValueFieldProvider } from "./index.ts";

/** Draw a value's input, recording what it gives. */
function draw(type: ActionValue, start?: JsonValue, choices: readonly Choice[] = []) {
    const given: (JsonValue | undefined)[] = [];
    const [value, setValue] = createSignal(start);
    const { container } = render(() => (
        <ValueField
            type={type}
            value={value()}
            choices={choices}
            onValueChange={(next) => {
                given.push(next);
                setValue(() => next);
            }}
        />
    ));
    const write = (text: string) => {
        const input = present(
            container.querySelector<HTMLInputElement | HTMLTextAreaElement>("input, textarea"),
            "the input",
        );
        input.value = text;
        input.dispatchEvent(new InputEvent("input", { bubbles: true }));
        flush();
    };

    return { container, given, type: write };
}

test("give text, numbers and JSON", () => {
    const text = draw({ kind: "text" });
    text.type("Lake");
    text.type("");
    const number = draw({ kind: "number", format: { style: "percent" } });
    number.type("25%");
    const json = draw({ kind: "json", schema: describe(schema.json()) });
    json.type('{"count": 1}');
    json.type("{");

    expect([text.given, number.given, json.given]).toEqual([
        ["Lake", undefined],
        [0.25],
        [{ count: 1 }, undefined],
    ]);
});

test("give a yes or no from where it starts", () => {
    const { container, given } = draw({ kind: "boolean" }, true);
    container.querySelector<HTMLElement>("[role=switch], input[type=checkbox]")?.click();
    flush();

    expect(given).toEqual([false]);
});

test("take an input an app replaces", () => {
    const given: (JsonValue | undefined)[] = [];
    const { container } = render(() => (
        <ValueFieldProvider
            inputs={{
                number: (properties) => (
                    <button
                        type="button"
                        onClick={() => properties.onValueChange(properties.type.maximum)}
                    >
                        most
                    </button>
                ),
            }}
        >
            <ValueField
                type={{ kind: "number", maximum: 5 }}
                value={undefined}
                onValueChange={(value) => void given.push(value)}
            />
        </ValueFieldProvider>
    ));
    container.querySelector("button")?.click();

    expect(given).toEqual([5]);
});

test("show the title of a person, and keep them as other text is typed", () => {
    stubPopovers();
    const people = [{ value: "a", title: "Ada" }];
    const { container, given, type } = draw({ kind: "person" }, "a", people);
    const shown = () => container.querySelector("input")?.value;
    const before = shown();
    type("Gr");
    const during = shown();
    type("");

    expect([before, during, shown(), given]).toEqual(["Ada", "Gr", "", [undefined]]);
});

test("keep JSON as it is typed", () => {
    const { container, type } = draw({ kind: "json", schema: describe(schema.json()) }, [1]);
    const shown = () => container.querySelector("textarea")?.value;
    const before = shown();
    type("[1,  2]");
    const during = shown();
    type("[1,");

    expect([before, during, shown()]).toEqual(["[\n  1\n]", "[1,  2]", "[1,"]);
});

test("give a color typed in its picker, the swatch and text showing it", () => {
    stubPopovers();
    const { container, given } = draw({ kind: "json", schema: describe(Color.schema) }, "#f76b15");

    // open the picker and type a color into its text
    container.querySelector<HTMLButtonElement>("button")?.click();
    flush();
    const field = present(
        document.querySelector<HTMLInputElement>("input[data-slot=color-field]"),
        "the color's text",
    );
    field.value = "#0090ff";
    field.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    flush();

    expect([container.querySelector("bdi")?.textContent, given.at(-1)]).toEqual([
        "#0090ff",
        "#0090ff",
    ]);
});

test("add, move and remove the colors of a palette between the fewest and the most", () => {
    stubPopovers();
    const { container, given } = draw({ kind: "json", schema: describe(Palette.min(2).max(3)) }, [
        "#e0eaff",
        "#241d9a",
    ]);

    // adding repeats the last color up to the most, a swatch's popover moves its color and removes it
    press(container, "Add color");
    press(container, "Add color");
    press(document.body, "Move later", "Color 1");
    press(document.body, "Remove color", "Color 3");

    expect(given).toEqual([
        ["#e0eaff", "#241d9a", "#241d9a"],
        ["#241d9a", "#e0eaff", "#241d9a"],
        ["#241d9a", "#e0eaff"],
    ]);
});

/** Read a button's name without the marks that isolate a number in it. */
function nameOf(button: HTMLButtonElement): string {
    return (button.getAttribute("aria-label") ?? button.textContent).replaceAll(
        /[\u2066-\u2069]/gu,
        "",
    );
}

/** Press the button of a name, inside the popover of another name where one is given. */
function press(root: Element, label: string, popover?: string): void {
    const scope =
        popover === undefined
            ? root
            : present(
                  root.querySelector(`[data-slot=popover-content][aria-label="${popover}"]`),
                  `the popover of ${popover}`,
              );
    const buttons = [...scope.querySelectorAll<HTMLButtonElement>("button")];
    present(
        buttons.find((each) => nameOf(each) === label),
        `the button ${label}`,
    ).click();
    flush();
}

/** Describe a schema as the JSON Schema a JSON input carries. */
function describe(definition: schema.Schema): JsonObject {
    return JsonValue.object(toJsonSchema(definition));
}
