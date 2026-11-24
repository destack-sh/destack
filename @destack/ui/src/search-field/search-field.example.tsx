import { defineExample } from "@destack/package/declare";
import { createSignal, For } from "@destack/view";
import { SearchField } from "./search-field.tsx";

/** The notebooks to search. */
const NOTEBOOKS = ["Trips", "Work", "Recipes", "Garden"];

/** A list narrowed by what a person types. */
export const searchFieldNotebooks = defineExample({
    of: SearchField,
    name: "notebooks",
    description: "a list narrowed by what a person types",
    render: () => {
        const [text, setText] = createSignal("");
        const found = () =>
            NOTEBOOKS.filter((name) => name.toLowerCase().includes(text().toLowerCase()));

        return (
            <div>
                <SearchField value={text()} onValueChange={setText} placeholder="Find a notebook" />
                <ul>
                    <For each={found()}>{(name) => <li>{name}</li>}</For>
                </ul>
            </div>
        );
    },
});
