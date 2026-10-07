import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { Checkbox } from "./index.ts";
import { draw, markup } from "@destack/view/test";

/** List the checked state of each input of a container, in order. */
function checked(container: Element): boolean[] {
    return [...container.querySelectorAll("input")].map((input) => input.checked);
}

test("toggle a native checkbox on click, and show an indeterminate state", () => {
    const container = draw(() => (
        <>
            <Checkbox name="terms" />
            <Checkbox indeterminate aria-label="Select all" />
            <Checkbox disabled />
        </>
    ));
    container.querySelector("input")?.click();
    flush();

    // the first box toggles, and the second reports indeterminate through its property
    expect(markup(container)).toBe(
        '<input type="checkbox" data-slot="checkbox" data-state="checked" name="terms">' +
            '<input type="checkbox" data-slot="checkbox" data-state="indeterminate" aria-label="Select all">' +
            '<input type="checkbox" data-slot="checkbox" data-state="unchecked" disabled="">',
    );
    expect(checked(container)).toEqual([true, false, false]);
    expect([...container.querySelectorAll("input")].map((input) => input.indeterminate)).toEqual([
        false,
        true,
        false,
    ]);
});
