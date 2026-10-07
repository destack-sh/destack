import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { RadioGroup, RadioGroupItem } from "./index.ts";
import { draw, markup } from "@destack/view/test";

/** List the checked state of each input of a container, in order. */
function checked(container: Element): boolean[] {
    return [...container.querySelectorAll("input")].map((input) => input.checked);
}

test("share one name across a radio group's radios, checking one at a time", () => {
    const changes: string[] = [];
    const container = draw(() => (
        <RadioGroup
            name="density"
            aria-label="Density"
            defaultValue="compact"
            onValueChange={(value) => changes.push(value)}
        >
            <RadioGroupItem value="compact" aria-label="Compact" />
            <RadioGroupItem value="regular" aria-label="Regular" />
        </RadioGroup>
    ));
    container.querySelectorAll("input")[1]?.click();
    flush();
    expect(markup(container)).toBe(
        '<div role="radiogroup" data-slot="radio-group" aria-label="Density">' +
            '<input type="radio" name="density" value="compact" data-slot="radio-group-item" data-state="unchecked" aria-label="Compact">' +
            '<input type="radio" name="density" value="regular" data-slot="radio-group-item" data-state="checked" aria-label="Regular"></div>',
    );
    expect([checked(container), changes]).toEqual([[false, true], ["regular"]]);
});

test("hold a controlled radio group at its owner's value until the owner changes it", () => {
    const container = draw(() => (
        <RadioGroup aria-label="Density" value="compact">
            <RadioGroupItem value="compact" aria-label="Compact" />
            <RadioGroupItem value="regular" aria-label="Regular" />
        </RadioGroup>
    ));
    container.querySelectorAll("input")[1]?.click();
    flush();
    expect(checked(container)).toEqual([true, false]);
});

test("disable and require every radio of a group", () => {
    const container = draw(() => (
        <RadioGroup aria-label="Density" disabled required>
            <RadioGroupItem value="compact" aria-label="Compact" />
        </RadioGroup>
    ));
    const radio = container.querySelector("input");
    expect([radio?.disabled, radio?.required]).toEqual([true, true]);
});
