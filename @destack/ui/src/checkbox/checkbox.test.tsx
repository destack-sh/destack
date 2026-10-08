import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { Checkbox, CheckboxIndicator } from "./index.ts";
import { markup, render } from "@destack/view/test";

/** List the state each checkbox of a container reports, in order. */
function checked(container: Element): string[] {
    return [...container.querySelectorAll("[role=checkbox]")].map(
        (box) => box.getAttribute("aria-checked") ?? "",
    );
}

test("toggle a checkbox and its indicator on click, report an indeterminate one as mixed, and ignore a disabled one", () => {
    const { container } = render(() => (
        <>
            <Checkbox aria-label="Terms">
                <CheckboxIndicator />
            </Checkbox>
            <Checkbox indeterminate aria-label="Select all" />
            <Checkbox disabled aria-label="Locked" />
        </>
    ));
    for (const box of container.querySelectorAll<HTMLElement>("[role=checkbox]")) {
        box.click();
    }
    flush();

    expect(markup(container)).toBe(
        '<button type="button" role="checkbox" aria-checked="true" data-slot="checkbox" data-state="checked" aria-label="Terms"><span data-slot="checkbox-indicator" data-state="checked"><svg viewBox="0 0 256 256" fill="currentColor" width="1em" height="1em" aria-hidden="true" data-slot="icon"></svg></span></button>' +
            '<button type="button" role="checkbox" aria-checked="mixed" data-slot="checkbox" data-state="indeterminate" aria-label="Select all"><span data-slot="checkbox-indicator" data-state="indeterminate"><svg viewBox="0 0 256 256" fill="currentColor" width="1em" height="1em" aria-hidden="true" data-slot="icon"></svg></span></button>' +
            '<button type="button" role="checkbox" aria-checked="false" data-slot="checkbox" data-state="unchecked" data-disabled="" disabled="" aria-label="Locked"></button>',
    );
});

test("hold a controlled checkbox at its owner's state, submit it under its name, and restore it on a form reset", () => {
    const [isChosen, setChosen] = createSignal(false);
    const { container } = render(() => (
        <form>
            <Checkbox name="terms" defaultChecked aria-label="Terms" />
            <Checkbox aria-label="Fixed" checked={false} />
            <Checkbox aria-label="Followed" checked={isChosen()} onCheckedChange={setChosen} />
        </form>
    ));
    for (const box of container.querySelectorAll<HTMLElement>("[role=checkbox]")) {
        box.click();
    }
    flush();
    const submitted = container.querySelector("input");
    const clicked = [checked(container), submitted?.name, submitted?.checked];
    container.querySelector("form")?.reset();
    flush();

    expect([clicked, checked(container)]).toEqual([
        [["false", "false", "true"], "terms", false],
        ["true", "false", "true"],
    ]);
});
