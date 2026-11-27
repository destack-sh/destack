import { Duration, JsonValue, present, type schema, toJsonSchema } from "@destack/schema";
import { Color, Dimension, FontFamily, Reference } from "@destack/style";
import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import { preloadNamedInputs, ValueField } from "./index.ts";

// load the kit's named inputs, so a field of a bindable schema renders at once
await preloadNamedInputs();

/** Draw a value that binds to a token where some tokens are set, recording what it gives. */
function draw(start: JsonValue, definition: schema.Schema = Dimension.bindable) {
    const given: JsonValue[] = [];
    const [value, setValue] = createSignal<JsonValue | undefined>(start);
    const { container } = render(() => (
        <div
            style={{
                "--destack-radius-2": "6px",
                "--destack-color-border": "#e6e6e6",
                "--destack-font-text": "Inter, sans-serif",
                "--destack-motion-duration-short": "150ms",
            }}
        >
            <ValueField
                type={{ kind: "json", schema: JsonValue.object(toJsonSchema(definition)) }}
                aria-label="Radius"
                value={value()}
                onValueChange={(next) => {
                    given.push(present(next, "a value"));
                    setValue(() => next);
                }}
            />
        </div>
    ));
    const click = (label: string): void => {
        present(container.querySelector<HTMLElement>(`[aria-label="${label}"]`), label).click();
        flush();
    };

    return { container, given, click };
}

test("bind a length to a token picked from its menu, then show the token", () => {
    stubPopovers();
    const { container, given, click } = draw({ value: 16, unit: "px" });

    // open the menu of dimension tokens, and pick one
    click("Bind to token");
    present(document.querySelector<HTMLElement>('[data-value="{space.4}"]'), "space.4").click();
    flush();

    expect([given, container.querySelector("[data-slot=token]")?.textContent]).toEqual([
        ["{space.4}"],
        "space.4",
    ]);
});

test("list first the tokens that draw the length a field holds, here 6px as radius.2", () => {
    stubPopovers();
    const { click } = draw({ value: 6, unit: "px" });
    click("Bind to token");
    const matching = [...document.querySelectorAll("[data-value^=Matching]")].map((item) =>
        item.getAttribute("data-value"),
    );

    expect(matching).toEqual(["Matching {radius.2}"]);
});

test("detach a token into the length it draws where the field is", () => {
    const { given, click } = draw("{radius.2}");
    click("Detach from token");

    expect(given).toEqual([{ value: 6, unit: "px" }]);
});

test("offer to bind again once a token is detached, in every field that binds", () => {
    // detach a length, a color, fonts and a duration, then look for each field's bind button
    stubPopovers();
    const cases = [
        ["{radius.2}", Dimension.bindable],
        ["{color.border}", Color.bindable],
        ["{font.text}", FontFamily.bindable],
        ["{motion.duration.short}", Reference.or(Duration, "duration")],
    ] as const;
    const after = cases.map(([token, definition]) => {
        const { container, click } = draw(token, definition);
        click("Detach from token");

        return container.querySelector('[aria-label="Bind to token"]') !== null;
    });

    expect(after).toEqual([true, true, true, true]);
});
