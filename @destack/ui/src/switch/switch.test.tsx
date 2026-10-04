import { expect, test } from "@destack/test";
import { Switch } from "./index.ts";
import { draw, markup } from "@destack/view/test";

/** List the checked state of each input of a container, in order. */
function checked(container: Element): boolean[] {
    return [...container.querySelectorAll("input")].map((input) => input.checked);
}

test("expose a native checkbox as a switch that toggles on click", () => {
    const container = draw(() => <Switch name="notifications" checked />);
    container.querySelector("input")?.click();
    expect(markup(container)).toBe(
        '<input type="checkbox" role="switch" data-slot="switch" name="notifications">',
    );
    expect(checked(container)).toEqual([false]);
});
