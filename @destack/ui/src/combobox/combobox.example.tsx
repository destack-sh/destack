import { t } from "@destack/locale";
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
    name: "moveNote",
    description: t`A combobox that picks the notebook a note moves to.`,
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
    name: "moveNoteOpen",
    description: t`The notebook combobox with its list of options open.`,
    properties: {
        defaultOpen: true,
    },
    render: (properties) => (
        <Combobox {...properties}>
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
    name: "tagNote",
    description: t`A combobox that tags a note, several tags at once and new ones from the typed text.`,
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
    name: "findPersonLoading",
    description: t`A combobox that searches people while their matches load.`,
    properties: {
        filter: null,
        defaultOpen: true,
    },
    render: (properties) => (
        <Combobox {...properties}>
            <ComboboxInput aria-label="Person" placeholder="Find a person" />
            <ComboboxContent>
                <ComboboxLoading>Searching…</ComboboxLoading>
            </ComboboxContent>
        </Combobox>
    ),
});

/** The notebook combobox marked invalid. */
export const comboboxMoveNoteInvalid = defineExample({
    of: Combobox,
    name: "moveNoteInvalid",
    description: t`The notebook combobox marked invalid.`,
    render: () => (
        <Combobox>
            <ComboboxInput
                aria-label="Notebook"
                placeholder="Move to notebook"
                aria-invalid="true"
            />
            <ComboboxContent>
                <ComboboxItem value="Trips">Trips</ComboboxItem>
                <ComboboxItem value="Work">Work</ComboboxItem>
            </ComboboxContent>
        </Combobox>
    ),
});

/** The notebook combobox unavailable. */
export const comboboxMoveNoteDisabled = defineExample({
    of: Combobox,
    name: "moveNoteDisabled",
    description: t`The notebook combobox unavailable.`,
    render: () => (
        <Combobox>
            <ComboboxInput aria-label="Notebook" placeholder="Move to notebook" disabled />
            <ComboboxContent>
                <ComboboxItem value="Trips">Trips</ComboboxItem>
            </ComboboxContent>
        </Combobox>
    ),
});

/** The notebooks a note moves to, the archive set on its own. */
export const comboboxItemDefault = defineExample({
    of: ComboboxItem,
    name: "default",
    description: t`The notebooks a note moves to, the archive set on its own.`,
    properties: { disabled: true },
    render: (properties) => (
        <Combobox defaultOpen>
            <ComboboxInput aria-label="Notebook" placeholder="Move to notebook" />
            <ComboboxContent>
                <ComboboxItem value="Trips">Trips</ComboboxItem>
                <ComboboxItem value="Archive" {...properties}>
                    Archive
                </ComboboxItem>
            </ComboboxContent>
        </Combobox>
    ),
});
