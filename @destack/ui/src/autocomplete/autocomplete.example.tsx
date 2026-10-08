import { defineExample } from "@destack/package/declare";
import { createSignal } from "@destack/view";
import { ListBox, ListBoxEmpty, ListBoxItem } from "../list-box/index.ts";
import { Autocomplete, AutocompleteInput } from "./autocomplete.tsx";

/** A search over a list box of fruits, reporting the fruit chosen with Enter or a click. */
export const autocompleteFruit = defineExample({
    of: Autocomplete,
    name: "fruit",
    description:
        "a search over a list box of fruits, reporting the fruit chosen with Enter or a click",
    render: () => {
        const [fruit, setFruit] = createSignal("");

        return (
            <>
                <Autocomplete>
                    <AutocompleteInput aria-label="Fruit" placeholder="Find a fruit" />
                    <ListBox aria-label="Fruits" onAction={(option) => setFruit(option.value())}>
                        <ListBoxEmpty>No fruit found</ListBoxEmpty>
                        <ListBoxItem>Apple</ListBoxItem>
                        <ListBoxItem>Banana</ListBoxItem>
                        <ListBoxItem keywords={["tropical"]}>Mango</ListBoxItem>
                    </ListBox>
                </Autocomplete>
                <output>{fruit()}</output>
            </>
        );
    },
});

/** A search that finds no fruit, showing the list box's empty note. */
export const autocompleteNoFruit = defineExample({
    of: Autocomplete,
    name: "no-fruit",
    description: "a search that finds no fruit, showing the list box's empty note",
    render: () => (
        <Autocomplete defaultSearch="kiwi">
            <AutocompleteInput aria-label="Fruit" />
            <ListBox aria-label="Fruits">
                <ListBoxEmpty>No fruit found</ListBoxEmpty>
                <ListBoxItem>Apple</ListBoxItem>
            </ListBox>
        </Autocomplete>
    ),
});
