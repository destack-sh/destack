import { defineExample } from "@destack/package/declare";
import {
    Combobox,
    ComboboxContent,
    ComboboxEmpty,
    ComboboxInput,
    ComboboxItem,
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
