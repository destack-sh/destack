import type { JSX } from "@solidjs/web";
import {
    Combobox,
    ComboboxContent,
    ComboboxEmpty,
    ComboboxInput,
    ComboboxItem,
} from "./combobox.tsx";

/** Show a combobox that picks the notebook a note moves to. */
export function ComboboxExample(): JSX.Element {
    return (
        <Combobox>
            <ComboboxInput aria-label="Notebook" placeholder="Move to notebook" />
            <ComboboxContent>
                <ComboboxEmpty>No notebook found</ComboboxEmpty>
                <ComboboxItem value="Trips">Trips</ComboboxItem>
                <ComboboxItem value="Work">Work</ComboboxItem>
                <ComboboxItem value="Recipes">Recipes</ComboboxItem>
            </ComboboxContent>
        </Combobox>
    );
}
