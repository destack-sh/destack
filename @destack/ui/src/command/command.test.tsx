import { expect, test } from "@destack/test";
import { flush } from "solid-js";
import { draw } from "@destack/view/test";
import { commandNoteCommands } from "./command.example.tsx";

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

test("filter a command list by the search and its keywords, hiding empty groups", () => {
    const container = draw(commandNoteCommands);
    const input = find(container, "input", HTMLInputElement);
    const all = shown(container);
    typeInto(input, "TRASH");
    const matched = shown(container);
    const hidden = [...container.querySelectorAll("[role=group]")].map((group) =>
        group.hasAttribute("hidden"),
    );
    typeInto(input, "zebra");

    // the first enabled match is highlighted, and the input points at it
    expect([
        all,
        matched,
        hidden,
        shown(container),
        container.textContent?.includes("No results"),
    ]).toEqual([
        ["[New note]", "Delete note", "Billing", "Appearance"],
        ["[Delete note]"],
        [false, true],
        [],
        true,
    ]);
});
