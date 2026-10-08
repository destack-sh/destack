import { expect, onTestFinished, test } from "@destack/test";
import { createRoot, flush } from "@destack/view";
import { render } from "@destack/view/test";
import { ListBox, ListBoxItem } from "../list-box/index.ts";
import { Autocomplete, AutocompleteControl, AutocompleteInput } from "./index.ts";

/** Create an autocomplete disposed after the test. */
function createAutocomplete(
    properties: ConstructorParameters<typeof AutocompleteControl>[0],
): AutocompleteControl {
    return createRoot((dispose) => {
        onTestFinished(dispose);

        return new AutocompleteControl(properties);
    });
}

/** Read whether three fruits match an autocomplete's search. */
function matched(control: AutocompleteControl): boolean[] {
    return [
        control.matches("Mango", []),
        control.matches("Banana", ["tropical"]),
        control.matches("Apple", ["smash"]),
    ];
}

test("match items by their words holding the search, by the owner's filter, or all without filtering", () => {
    // search for "ma" three ways
    const words = createAutocomplete({ defaultSearch: " MA " });
    const filtered = createAutocomplete({
        defaultSearch: "ma",
        filter: (value, search) => value.startsWith(search),
    });
    const unfiltered = createAutocomplete({ defaultSearch: "ma", shouldFilter: false });

    expect({
        words: matched(words),
        filtered: matched(filtered),
        unfiltered: matched(unfiltered),
        empty: createAutocomplete({}).matches("Apple", []),
    }).toEqual({
        words: [true, false, true],
        filtered: [false, false, false],
        unfiltered: [true, true, true],
        empty: true,
    });
});

test("drive the list box inside from the search field: its id, its focused option and a new search's first match", () => {
    // render a search over three fruits and move down once
    const { container } = render(() => (
        <Autocomplete>
            <AutocompleteInput aria-label="Fruit" />
            <ListBox aria-label="Fruits">
                <ListBoxItem>Apple</ListBoxItem>
                <ListBoxItem>Banana</ListBoxItem>
                <ListBoxItem>Mango</ListBoxItem>
            </ListBox>
        </Autocomplete>
    ));
    flush();
    const input = container.querySelector("input");
    const press = (key: string): boolean => {
        const event = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true });
        input?.dispatchEvent(event);
        flush();

        return event.defaultPrevented;
    };
    const taken = [press("ArrowDown"), press("ArrowLeft"), press("Home")];
    const focused = () =>
        document.getElementById(input?.getAttribute("aria-activedescendant") ?? "");
    const moved = focused()?.textContent;
    if (input !== null) {
        input.value = "an";
        input.dispatchEvent(new InputEvent("input", { bubbles: true }));
    }
    flush();

    // the caret keeps the left and right arrows, and a new search focuses its first match
    expect({
        controls:
            input?.getAttribute("aria-controls") === container.querySelector("[role=listbox]")?.id,
        taken,
        moved,
        searched: focused()?.textContent,
    }).toEqual({ controls: true, taken: [true, false, true], moved: "Apple", searched: "Banana" });
});
