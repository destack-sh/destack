import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { render } from "@destack/view/test";
import {
    Combobox,
    ComboboxChip,
    ComboboxChipRemove,
    ComboboxChips,
    ComboboxInput,
    ComboboxItem,
    ComboboxItemIndicator,
} from "./index.ts";

test("follow a controlled input text, reporting what the person types and filtering by the owner's text", () => {
    const [text, setText] = createSignal("wo");
    const { container } = render(() => (
        <Combobox inputValue={text()} onInputValueChange={setText}>
            <ComboboxInput aria-label="Notebook" />
            <ComboboxItem value="Trips">Trips</ComboboxItem>
            <ComboboxItem value="Work">Work</ComboboxItem>
        </Combobox>
    ));
    const input = container.querySelector("input");
    const shown = () =>
        [...container.querySelectorAll("[role=option]:not([hidden])")].map(
            (option) => option.textContent,
        );
    const owned = [input?.value, shown()];
    if (input !== null) {
        input.value = "tr";
        input.dispatchEvent(new InputEvent("input", { bubbles: true }));
    }
    flush();

    expect([owned, text(), shown()]).toEqual([["wo", ["Work"]], "tr", ["Trips"]]);
});

test("drop a chosen value from the remove button of the caller's own chip", () => {
    const changes: (readonly string[])[] = [];
    const { container } = render(() => (
        <Combobox
            multiple
            defaultValue={["travel", "food"]}
            onValueChange={(values) => changes.push(values)}
        >
            <ComboboxChips>
                <ComboboxChip value="food">
                    food <ComboboxChipRemove>×</ComboboxChipRemove>
                </ComboboxChip>
            </ComboboxChips>
            <ComboboxInput aria-label="Tags" />
        </Combobox>
    ));
    container.querySelector<HTMLElement>("[data-slot=combobox-chip-remove]")?.click();
    flush();

    expect(changes).toEqual([["travel"]]);
});

test("check the chosen item's indicator and move it as the person chooses another", () => {
    const { container } = render(() => (
        <Combobox defaultValue="Work">
            <ComboboxInput aria-label="Notebook" />
            <ComboboxItem value="Trips">
                Trips
                <ComboboxItemIndicator />
            </ComboboxItem>
            <ComboboxItem value="Work">
                Work
                <ComboboxItemIndicator />
            </ComboboxItem>
        </Combobox>
    ));
    const checked = () =>
        [...container.querySelectorAll("[data-slot=combobox-item-indicator]")].map(
            (indicator) => indicator.closest("[role=option]")?.textContent,
        );
    const before = checked();
    container.querySelector<HTMLElement>("[role=option]")?.click();
    flush();

    expect([before, checked()]).toEqual([["Work"], ["Trips"]]);
});
