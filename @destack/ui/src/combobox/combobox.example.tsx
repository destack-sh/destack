import { defineExample } from "@destack/package/declare";
import {
    Combobox,
    ComboboxChips,
    ComboboxContent,
    ComboboxCreate,
    ComboboxEmpty,
    ComboboxInput,
    ComboboxItem,
    ComboboxLoading,
} from "./combobox.tsx";

/** A combobox that picks the notebook a note moves to. */
export const comboboxMoveNote = defineExample({
    of: Combobox,
    name: "move-note",
    description: "a combobox that picks the notebook a note moves to",
    render: () => (
        <Combobox>
            <ComboboxInput aria-label="Notebook" placeholder="Move to notebook" />
            <ComboboxContent>
                <ComboboxEmpty>No notebook found</ComboboxEmpty>
                <ComboboxItem value="Trips">Trips</ComboboxItem>
                <ComboboxItem value="Work">Work</ComboboxItem>
                <ComboboxItem value="Recipes">Recipes</ComboboxItem>
            </ComboboxContent>
        </Combobox>
    ),
});

/** The notebook combobox with its list of options open. */
export const comboboxMoveNoteOpen = defineExample({
    of: Combobox,
    name: "move-note-open",
    description: "the notebook combobox with its list of options open",
    render: () => (
        <Combobox defaultOpen>
            <ComboboxInput aria-label="Notebook" placeholder="Move to notebook" />
            <ComboboxContent>
                <ComboboxEmpty>No notebook found</ComboboxEmpty>
                <ComboboxItem value="Trips">Trips</ComboboxItem>
                <ComboboxItem value="Work">Work</ComboboxItem>
                <ComboboxItem value="Recipes">Recipes</ComboboxItem>
            </ComboboxContent>
        </Combobox>
    ),
});

/** A combobox that tags a note, several tags at once and new ones from the typed text. */
export const comboboxTagNote = defineExample({
    of: Combobox,
    name: "tag-note",
    description:
        "a combobox that tags a note, several tags at once and new ones from the typed text",
    render: () => (
        <Combobox multiple defaultValue={["travel"]}>
            <ComboboxChips />
            <ComboboxInput aria-label="Tags" placeholder="Add a tag" />
            <ComboboxContent>
                <ComboboxItem value="travel">travel</ComboboxItem>
                <ComboboxItem value="food">food</ComboboxItem>
                <ComboboxItem value="family">family</ComboboxItem>
                <ComboboxCreate />
            </ComboboxContent>
        </Combobox>
    ),
});

/** A combobox that searches people while their matches load. */
export const comboboxFindPersonLoading = defineExample({
    of: Combobox,
    name: "find-person-loading",
    description: "a combobox that searches people while their matches load",
    render: () => (
        <Combobox shouldFilter={false} defaultOpen>
            <ComboboxInput aria-label="Person" placeholder="Find a person" />
            <ComboboxContent>
                <ComboboxLoading>Searching…</ComboboxLoading>
            </ComboboxContent>
        </Combobox>
    ),
});
