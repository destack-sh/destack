import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { Switch } from "./index.ts";
import { draw, markup } from "@destack/view/test";

/** List the checked state of each input of a container, in order. */
function checked(container: Element): boolean[] {
    return [...container.querySelectorAll("input")].map((input) => input.checked);
}

test("expose a native checkbox as a switch that toggles on click", () => {
    const changes: boolean[] = [];
    const container = draw(() => (
        <Switch name="notifications" defaultChecked onCheckedChange={(on) => changes.push(on)} />
    ));
    container.querySelector("input")?.click();
    flush();
    expect(markup(container)).toBe(
        '<input type="checkbox" role="switch" data-slot="switch" data-state="unchecked" name="notifications">',
    );
    expect([checked(container), changes]).toEqual([[false], [false]]);
});

test("hold a controlled switch at its owner's state until the owner changes it", () => {
    const [isOn, setOn] = createSignal(true);
    const container = draw(() => (
        <>
            <Switch aria-label="Fixed" checked />
            <Switch aria-label="Followed" checked={isOn()} onCheckedChange={setOn} />
        </>
    ));
    for (const input of container.querySelectorAll("input")) {
        input.click();
    }
    flush();
    expect([checked(container), isOn()]).toEqual([[true, false], false]);
});
