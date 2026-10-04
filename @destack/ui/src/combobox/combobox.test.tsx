import { expect, test } from "@destack/test";
import { flush } from "solid-js";
import { Combobox, ComboboxContent, ComboboxEmpty, ComboboxInput, ComboboxItem } from "./index.ts";
import { draw, press, stubPopovers } from "@destack/view/test";

/** Find the element of a container's first match, refusing none. */
function find<Target extends HTMLElement>(
    container: Element,
    selector: string,
    type: new () => Target,
): Target {
    const element = container.querySelector(selector);
    if (!(element instanceof type)) {
        throw new TypeError(`no ${type.name} matches ${selector}`);
    }

    return element;
}

/** Type text into an input as a person does. */
function typeInto(input: HTMLInputElement, text: string): void {
    input.value = text;
    input.dispatchEvent(new InputEvent("input", { bubbles: true }));
    flush();
}

/** List the text of the options shown, marking the highlighted one. */
function shown(container: Element): string[] {
    return [...container.querySelectorAll<HTMLElement>("[role=option]:not([hidden])")].map(
        (option) =>
            option.getAttribute("aria-selected") === "true"
                ? `[${option.textContent}]`
                : (option.textContent ?? ""),
    );
}

test("open a combobox's list as the person types, choose an option and close it", () => {
    stubPopovers();
    const chosen: string[] = [];
    const container = draw(() => (
        <Combobox onValueChange={(value) => chosen.push(value)}>
            <ComboboxInput aria-label="Notebook" />
            <ComboboxContent>
                <ComboboxEmpty>No notebook</ComboboxEmpty>
                <ComboboxItem value="Trips">Trips</ComboboxItem>
                <ComboboxItem value="Work">Work</ComboboxItem>
            </ComboboxContent>
        </Combobox>
    ));
    const input = find(container, "input", HTMLInputElement);
    const list = find(container, "[role=listbox]", HTMLElement);
    input.focus();
    typeInto(input, "wo");
    const opened = [
        input.getAttribute("aria-expanded"),
        list.getAttribute("data-popover-open"),
        shown(container),
    ];
    press("Enter");
    flush();

    // the chosen value fills the input and the list closes
    expect(opened).toEqual(["true", "combobox-input", ["[Work]"]]);
    expect([
        chosen,
        input.value,
        input.getAttribute("aria-expanded"),
        list.hasAttribute("data-popover-open"),
    ]).toEqual([["Work"], "Work", "false", false]);
});

test("open a combobox's list with the down arrow key and close it with Escape, then clear the input", () => {
    stubPopovers();
    const container = draw(() => (
        <Combobox>
            <ComboboxInput aria-label="Notebook" />
            <ComboboxContent>
                <ComboboxItem value="Trips">Trips</ComboboxItem>
            </ComboboxContent>
        </Combobox>
    ));
    const input = find(container, "input", HTMLInputElement);
    input.focus();
    typeInto(input, "tr");
    press("Escape");
    flush();
    const closed = [input.getAttribute("aria-expanded"), input.value];
    press("ArrowDown");
    flush();
    const reopened = input.getAttribute("aria-expanded");
    press("Escape");
    flush();
    press("Escape");
    flush();
    expect([closed, reopened, input.value]).toEqual([["false", "tr"], "true", ""]);
});
