import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { draw } from "@destack/view/test";
import { commandNoteCommands } from "./command.example.tsx";
import {
    Command,
    CommandGroup,
    CommandGroupHeading,
    CommandInput,
    CommandItem,
    CommandList,
} from "./index.ts";

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

/** Press a key in an input. */
function press(input: HTMLInputElement, key: string): void {
    input.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
    flush();
}

test("follow a controlled search, reporting what the person types, and match by the owner's filter", () => {
    const [search, setSearch] = createSignal("cal");
    const container = draw(() => (
        <Command
            search={search()}
            onSearchChange={setSearch}
            filter={(value, typed) => value.toLowerCase().startsWith(typed.toLowerCase())}
        >
            <CommandInput />
            <CommandList>
                <CommandItem>Calendar</CommandItem>
                <CommandItem>Local time</CommandItem>
                <CommandItem forceMount>Help</CommandItem>
            </CommandList>
        </Command>
    ));
    const input = find(container, "input", HTMLInputElement);
    const owned = [input.value, shown(container)];
    typeInto(input, "loc");

    expect([owned, search(), shown(container)]).toEqual([
        ["cal", ["[Calendar]", "Help"]],
        "loc",
        ["[Local time]", "Help"],
    ]);
});

test("start on the default highlight, jump to the ends with Home and End, and wrap in a looping list", () => {
    const container = draw(() => (
        <Command defaultValue="Two" loop>
            <CommandInput />
            <CommandList>
                <CommandGroup>
                    <CommandGroupHeading>Numbers</CommandGroupHeading>
                    <CommandItem>One</CommandItem>
                    <CommandItem>Two</CommandItem>
                    <CommandItem>Three</CommandItem>
                </CommandGroup>
            </CommandList>
        </Command>
    ));
    const input = find(container, "input", HTMLInputElement);
    const highlighted = () => container.querySelector("[aria-selected=true]")?.textContent ?? "";
    const first = highlighted();
    press(input, "End");
    const last = highlighted();
    press(input, "ArrowDown");
    const wrapped = highlighted();
    press(input, "Home");
    const group = container.querySelector("[role=group]");
    const heading = container.querySelector("[data-slot=command-group-heading]");

    expect([first, last, wrapped, highlighted(), group?.getAttribute("aria-labelledby")]).toEqual([
        "Two",
        "Three",
        "One",
        "One",
        heading?.id,
    ]);
});
