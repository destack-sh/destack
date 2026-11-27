import {
    Duration,
    JsonValue,
    Money,
    present,
    Rating,
    RecurrenceRule,
    schema,
    toJsonSchema,
    Url,
} from "@destack/schema";
import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import { ValueField } from "./index.ts";

/** Draw a named value's input from a value, recording what it gives. */
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

/** Type text into the input of a name. */
function type(root: Element, label: string, text: string, event = "input"): void {
    const found = present(root.querySelector(`[aria-label="${label}"]`), `the field ${label}`);
    const input = present(
        found instanceof HTMLInputElement ? found : found.querySelector("input"),
        `the input ${label}`,
    );
    input.value = text;
    input.dispatchEvent(new Event(event, { bubbles: true }));
    flush();
}

/** Press the button of a name. */
function press(root: Element, label: string): void {
    const button = [...root.querySelectorAll<HTMLButtonElement>("button")].find(
        (each) => each.getAttribute("aria-label") === label,
    );
    present(button, `the button ${label}`).click();
    flush();
}

test("give a duration by the units typed, leaving out those set to none", () => {
    const { container, given } = draw(Duration, { hours: 1 });
    type(container, "Minutes", "30");
    type(container, "Hours", "0");

    expect(given).toEqual([{ hours: 1, minutes: 30 }, { minutes: 30 }]);
});

test("repeat every other week on the weekdays pressed, every week by default", () => {
    const { container, given } = draw(RecurrenceRule, { frequency: "weekly" });
    type(container, "Every", "2");
    press(container, "Monday");
    press(container, "Friday");
    type(container, "Every", "1");

    expect(given).toEqual([
        { frequency: "weekly", interval: 2 },
        { frequency: "weekly", interval: 2, byDay: [{ day: 1 }] },
        { frequency: "weekly", interval: 2, byDay: [{ day: 1 }, { day: 5 }] },
        { frequency: "weekly", byDay: [{ day: 1 }, { day: 5 }] },
    ]);
});

test("give money in its currency's minor unit from the amount typed in its major unit", () => {
    const start: Money = { amount: 1250, currency: "EUR" };
    const { container, given } = draw(Money, start);
    type(container, "Amount", "19.99");

    expect(given).toEqual([{ amount: 1999, currency: "EUR" }]);
});

test("rate by the star pressed, out of the most the schema allows, and clear by pressing it again", () => {
    const { container, given } = draw(Rating.max(3), 2);
    const stars = container.querySelectorAll("[data-slot=toggle-group-item], button");
    press(container, "3 stars");
    press(container, "3 stars");

    expect({ stars: stars.length, given }).toEqual({ stars: 3, given: [3, 0] });
});

test("give a web address once it parses, and none once the field is cleared", () => {
    const { container, given } = draw(Url, undefined);
    const input = present(
        container.querySelector<HTMLInputElement>("input[type=url]"),
        "the address",
    );
    for (const text of ["example", " https://example.com/notes ", ""]) {
        input.value = text;
        input.dispatchEvent(new Event("change", { bubbles: true }));
        flush();
    }

    expect(given).toEqual(["https://example.com/notes", undefined]);
});
