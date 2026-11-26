import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { createSignal, For } from "@destack/view";
import { SearchField } from "./search-field.tsx";

/** The notebooks to search. */
const NOTEBOOKS = ["Trips", "Work", "Recipes", "Garden"];

/** A list narrowed by what a person types. */
export const searchFieldNotebooks = defineExample({
    of: SearchField,
    name: "notebooks",
    description: t`A list narrowed by what a person types.`,
    properties: { placeholder: "Find a notebook" },
    render: (properties) => {
        const [text, setText] = createSignal("");
        const found = () =>
            NOTEBOOKS.filter((name) => name.toLowerCase().includes(text().toLowerCase()));

        return (
            <div>
                <SearchField {...properties} value={text()} onValueChange={setText} />
                <ul>
                    <For each={found()}>{(name) => <li>{name}</li>}</For>
                </ul>
            </div>
        );
    },
});
