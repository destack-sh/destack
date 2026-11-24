import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import {
    Combobox,
    ComboboxChip,
    ComboboxChipRemove,
    ComboboxChips,
    ComboboxInput,
    ComboboxItem,
    ComboboxItemIndicator,
} from "./index.ts";

test("show and hide every option from the trigger, the focus in the input", () => {
    // press the trigger twice without typing
    const { container } = render(() => (
        <Combobox>
            <ComboboxInput aria-label="Notebook" />
            <ComboboxItem value="trips">Trips</ComboboxItem>
        </Combobox>
    ));
    const input = container.querySelector("[data-slot=combobox-input]");
    const trigger = container.querySelector("[data-slot=combobox-trigger]");
    const press = () => {
        trigger?.dispatchEvent(
            new PointerEvent("pointerdown", { bubbles: true, cancelable: true }),
        );
        flush();

        return [input?.getAttribute("aria-expanded"), document.activeElement === input];
    };

    expect([press(), press()]).toEqual([
        ["true", true],
        ["false", true],
    ]);
});

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

test("show the chosen value's text, filter by what is typed, and show it again when left", () => {
    const chosen: unknown[] = [];
    const { container } = render(() => (
        <Combobox defaultValue="id-work" onValueChange={(value) => void chosen.push(value)}>
            <ComboboxInput aria-label="Notebook" />
            <ComboboxItem value="id-trips">Trips</ComboboxItem>
            <ComboboxItem value="id-work">Work</ComboboxItem>
        </Combobox>
    ));
    flush();
    const input = container.querySelector("input");
    const read = () => [
        input?.value,
        ...[...container.querySelectorAll("[role=option]:not([hidden])")].map(
            (option) => option.textContent,
        ),
    ];
    const type = (text: string) => {
        if (input !== null) {
            input.value = text;
            input.dispatchEvent(new InputEvent("input", { bubbles: true }));
        }
        flush();

        return read();
    };
    const leave = () => {
        input?.dispatchEvent(new FocusEvent("blur"));
        flush();

        return read();
    };

    expect([read(), type("tri"), leave(), type(""), leave(), chosen]).toEqual([
        ["Work", "Trips", "Work"],
        ["tri", "Trips"],
        ["Work", "Trips", "Work"],
        ["", "Trips", "Work"],
        ["", "Trips", "Work"],
        [undefined],
    ]);
});

test("open by a click on the input, its text ready to be typed over", () => {
    stubPopovers();
    const { container } = render(() => (
        <Combobox defaultValue="Work">
            <ComboboxInput aria-label="Notebook" />
            <ComboboxItem value="Work">Work</ComboboxItem>
        </Combobox>
    ));
    const input = container.querySelector("input");
    input?.click();
    flush();

    expect([
        input?.getAttribute("aria-expanded"),
        input?.selectionStart,
        input?.selectionEnd,
    ]).toEqual(["true", 0, 4]);
});
