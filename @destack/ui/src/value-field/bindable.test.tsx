import { JsonValue, present, toJsonSchema } from "@destack/schema";
import { Dimension } from "@destack/style";
import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import { preloadNamedInputs, ValueField } from "./index.ts";

// load the kit's named inputs, so a field of a bindable schema renders at once
await preloadNamedInputs();

/** Draw a length that binds to a token where one radius is set, recording what it gives. */
function draw(start: JsonValue) {
    const given: JsonValue[] = [];
    const [value, setValue] = createSignal<JsonValue | undefined>(start);
    const { container } = render(() => (
        <div style={{ "--destack-radius-2": "6px" }}>
            <ValueField
                type={{ kind: "json", schema: JsonValue.object(toJsonSchema(Dimension.bindable)) }}
                aria-label="Radius"
                value={value()}
                onValueChange={(next) => {
                    given.push(present(next, "a length"));
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

test("detach a token into the length it draws where the field is", () => {
    const { given, click } = draw("{radius.2}");
    click("Detach from token");

    expect(given).toEqual([{ value: 6, unit: "px" }]);
});
