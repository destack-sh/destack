import { Search } from "@destack/db";
import { defineExample } from "@destack/package/declare";
import { createSignal, For } from "@destack/view";
import { SearchField } from "../search-field/index.ts";
import { Stack } from "../stack/index.ts";
import { Highlight } from "./highlight.tsx";

/** The titles of some notes. */
const TITLES = ["Order form", "The order book", "Reorder the shelves", "Le Café des Orchidées"];

/** The notes a search finds, the best first, with what it matches marked. */
export const highlightNotes = defineExample({
    of: Highlight,
    name: "notes",
    description: "the notes a search finds, the best first, with what it matches marked",
    render: () => {
        const [text, setText] = createSignal("ord");
        const search = () => Search.of(text());

        return (
            <Stack space="3">
                <SearchField value={text()} onValueChange={setText} placeholder="Find a note" />
                <For each={search().rank(TITLES, (title) => [title])}>
                    {(title) => <Highlight text={title} search={search()} />}
                </For>
            </Stack>
        );
    },
});
