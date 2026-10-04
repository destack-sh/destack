import { expect, test } from "@destack/test";
import { flush } from "solid-js";
import { Toggle } from "./index.ts";
import { classes, draw, markup } from "@destack/view/test";

test("turn a toggle on and off, reporting its state", () => {
    const changes: boolean[] = [];
    const container = draw(() => (
        <Toggle aria-label="Bold" onPressedChange={(pressed) => changes.push(pressed)}>
            B
        </Toggle>
    ));
    const before = classes(container);
    container.querySelector("button")?.click();
    flush();

    // the pressed toggle styles differently and reports it
    expect(markup(container)).toBe(
        '<button type="button" data-slot="toggle" data-state="on" aria-pressed="true" aria-label="Bold">B</button>',
    );
    expect(classes(container)).not.toEqual(before);
    container.querySelector("button")?.click();
    flush();
    expect(changes).toEqual([true, false]);
});
