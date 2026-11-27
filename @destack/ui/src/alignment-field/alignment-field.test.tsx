import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { render } from "@destack/view/test";
import { type Alignment, AlignmentField } from "./index.ts";

test("choose a place by a click, then move it by the arrow keys within the grid", () => {
    // start at the top start, click the center, then walk right twice and down once
    const given: Alignment[] = [];
    const [value, setValue] = createSignal<Alignment>({ x: "start", y: "start" });
    const { container } = render(() => (
        <AlignmentField
            aria-label="Alignment"
            value={value()}
            onValueChange={(next) => {
                given.push(next);
                setValue(next);
            }}
        />
    ));
    const grid = present(container.querySelector<HTMLElement>("[role=radiogroup]"), "the grid");
    const press = (key: string): void => {
        grid.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
        flush();
    };
    present(grid.querySelector<HTMLElement>('[aria-label="Middle center"]'), "the center").click();
    flush();
    press("ArrowRight");
    press("ArrowRight");
    press("ArrowDown");

    // the second step right stops at the end and gives nothing, and the tab stop follows
    expect({
        given,
        checked: grid.querySelector("[aria-checked=true]")?.getAttribute("aria-label"),
        tabStop: grid.querySelector("[tabindex='0']")?.getAttribute("aria-label"),
    }).toEqual({
        given: [
            { x: "center", y: "center" },
            { x: "end", y: "center" },
            { x: "end", y: "end" },
        ],
        checked: "Bottom end",
        tabStop: "Bottom end",
    });
});
