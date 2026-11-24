import type { ActionValue, Choice } from "@destack/object";
import type { JsonValue } from "@destack/schema";
import { present } from "@destack/schema";
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
    const json = draw({ kind: "json" });
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
    const { container, type } = draw({ kind: "json" }, [1]);
    const shown = () => container.querySelector("textarea")?.value;
    const before = shown();
    type("[1,  2]");
    const during = shown();
    type("[1,");

    expect([before, during, shown()]).toEqual(["[\n  1\n]", "[1,  2]", "[1,"]);
});
