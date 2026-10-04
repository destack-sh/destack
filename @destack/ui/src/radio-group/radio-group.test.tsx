import { expect, test } from "@destack/test";
import { RadioGroup, RadioGroupItem } from "./index.ts";
import { draw, markup } from "@destack/view/test";

/** List the checked state of each input of a container, in order. */
function checked(container: Element): boolean[] {
    return [...container.querySelectorAll("input")].map((input) => input.checked);
}

test("share one name across a radio group's radios, checking one at a time", () => {
    const container = draw(() => (
        <RadioGroup name="density" aria-label="Density">
            <RadioGroupItem value="compact" checked aria-label="Compact" />
            <RadioGroupItem value="regular" aria-label="Regular" />
        </RadioGroup>
    ));
    container.querySelectorAll("input")[1]?.click();
    expect(markup(container)).toBe(
        '<div role="radiogroup" data-slot="radio-group" aria-label="Density">' +
            '<input type="radio" name="density" data-slot="radio-group-item" value="compact" aria-label="Compact">' +
            '<input type="radio" name="density" data-slot="radio-group-item" value="regular" aria-label="Regular"></div>',
    );
    expect(checked(container)).toEqual([false, true]);
});
