import { expect, test } from "@destack/test";
import { flush } from "solid-js";
import {
    Command,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandList,
    CommandSeparator,
} from "./index.ts";
import { draw, press } from "@destack/view/test";

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

/** Render a command palette with two groups. */
function drawPalette(chosen: string[]): HTMLElement {
    return draw(() => (
        <Command aria-label="Commands">
            <CommandInput placeholder="Type a command" />
            <CommandList>
                <CommandEmpty>No results</CommandEmpty>
                <CommandGroup heading="Notes">
                    <CommandItem onSelect={(value) => chosen.push(value)}>New note</CommandItem>
                    <CommandItem keywords={["trash"]} onSelect={(value) => chosen.push(value)}>
                        Delete note
                    </CommandItem>
                </CommandGroup>
                <CommandSeparator />
                <CommandGroup heading="Settings">
                    <CommandItem disabled>Billing</CommandItem>
                    <CommandItem value="theme" onSelect={(value) => chosen.push(value)}>
                        Appearance
                    </CommandItem>
                </CommandGroup>
            </CommandList>
        </Command>
    ));
}

test("filter a command list by the search and its keywords, hiding empty groups", () => {
    const container = drawPalette([]);
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

test("move the highlight with the arrow keys past disabled options, stopping at the ends, and choose with Enter", () => {
    const chosen: string[] = [];
    const container = drawPalette(chosen);
    const input = find(container, "input", HTMLInputElement);
    input.focus();
    const steps = ["ArrowDown", "ArrowDown", "ArrowDown", "ArrowUp"].map((key) => {
        press(key);
        flush();

        return shown(container).find((text) => text.startsWith("[")) ?? "";
    });
    press("Enter");
    flush();
    const active = container.querySelector("[aria-selected=true]")?.id;
    expect([steps, chosen, input.getAttribute("aria-activedescendant") === active]).toEqual([
        ["[Delete note]", "[Appearance]", "[Appearance]", "[Delete note]"],
        ["Delete note"],
        true,
    ]);
});

test("follow a controlled highlight and report the value the arrow keys move to", () => {
    const changes: string[] = [];
    const container = draw(() => (
        <Command value="Appearance" onValueChange={(value) => changes.push(value)}>
            <CommandInput />
            <CommandList>
                <CommandItem>New note</CommandItem>
                <CommandItem>Appearance</CommandItem>
            </CommandList>
        </Command>
    ));
    const highlighted = shown(container);
    container.querySelector("input")?.focus();
    press("ArrowUp");
    flush();

    // the owner's value stays highlighted until the owner changes it
    expect([highlighted, changes, shown(container)]).toEqual([
        ["New note", "[Appearance]"],
        ["New note"],
        ["New note", "[Appearance]"],
    ]);
});
