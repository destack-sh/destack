import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { RadioGroup, RadioGroupIndicator, RadioGroupItem } from "./index.ts";
import { draw, markup } from "@destack/view/test";

/** List the state each radio of a container reports, in order. */
function checked(container: Element): string[] {
    return [...container.querySelectorAll("[role=radio]")].map(
        (radio) => radio.getAttribute("aria-checked") ?? "",
    );
}

/** Press a key on the radio that holds the focus. */
function press(container: Element, key: string): void {
    const focused = container.ownerDocument.activeElement ?? container;
    focused.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
    flush();
}

test("check one radio at a time on click, showing its indicator and holding the tab stop", () => {
    const changes: string[] = [];
    const container = draw(() => (
        <RadioGroup
            aria-label="Density"
            defaultValue="compact"
            onValueChange={(value) => changes.push(value)}
        >
            <RadioGroupItem value="compact" aria-label="Compact" />
            <RadioGroupItem value="regular" aria-label="Regular">
                <RadioGroupIndicator />
            </RadioGroupItem>
        </RadioGroup>
    ));
    container.querySelectorAll<HTMLElement>("[role=radio]")[1]?.click();
    flush();
    expect(markup(container)).toBe(
        '<div role="radiogroup" data-slot="radio-group" aria-label="Density">' +
            '<button type="button" role="radio" aria-checked="false" tabindex="-1" data-slot="radio-group-item" data-state="unchecked" data-value="compact" aria-label="Compact"></button>' +
            '<button type="button" role="radio" aria-checked="true" tabindex="0" data-slot="radio-group-item" data-state="checked" data-value="regular" aria-label="Regular"><span data-slot="radio-group-indicator" data-state="checked"></span></button></div>',
    );
    expect(changes).toEqual(["regular"]);
});

test("move to and check the next enabled radio on an arrow key, wrapping at the ends", () => {
    const container = draw(() => (
        <RadioGroup aria-label="Density" defaultValue="compact">
            <RadioGroupItem value="compact" aria-label="Compact" />
            <RadioGroupItem value="cozy" aria-label="Cozy" disabled />
            <RadioGroupItem value="regular" aria-label="Regular" />
        </RadioGroup>
    ));
    container.querySelector<HTMLElement>("[role=radio]")?.focus();
    press(container, "ArrowDown");
    const forward = checked(container);
    press(container, "ArrowRight");

    expect([forward, checked(container)]).toEqual([
        ["false", "false", "true"],
        ["true", "false", "false"],
    ]);
});

test("hold a controlled radio group at its owner's value, and submit and reset a named one through hidden radios", () => {
    const controlled = draw(() => (
        <RadioGroup aria-label="Density" value="compact">
            <RadioGroupItem value="compact" aria-label="Compact" />
            <RadioGroupItem value="regular" aria-label="Regular" />
        </RadioGroup>
    ));
    controlled.querySelectorAll<HTMLElement>("[role=radio]")[1]?.click();
    flush();
    const named = draw(() => (
        <form>
            <RadioGroup name="density" aria-label="Density" defaultValue="compact" required>
                <RadioGroupItem value="compact" aria-label="Compact" />
                <RadioGroupItem value="regular" aria-label="Regular" />
            </RadioGroup>
        </form>
    ));
    named.querySelectorAll<HTMLElement>("[role=radio]")[1]?.click();
    flush();
    const submitted = [...named.querySelectorAll("input")].map((input) => [
        input.name,
        input.value,
        input.checked,
        input.required,
    ]);
    named.querySelector("form")?.reset();
    flush();

    expect([checked(controlled), submitted, checked(named)]).toEqual([
        ["true", "false"],
        [
            ["density", "compact", false, true],
            ["density", "regular", true, true],
        ],
        ["true", "false"],
    ]);
});
