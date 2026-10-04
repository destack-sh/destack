import type { JSX } from "@solidjs/web";
import { createSignal } from "solid-js";
import { Tree, TreeItem } from "./tree.tsx";

/** Show notebooks and their notes as a tree, opening a note on selection. */
export function TreeExample(): JSX.Element {
    const [note, setNote] = createSignal<string>();

    return (
        <Tree aria-label="Notebooks" value={note()} onValueChange={setNote}>
            <TreeItem value="trips" label="Trips" defaultExpanded>
                <TreeItem value="lisbon" label="Lisbon" />
                <TreeItem value="porto" label="Porto" />
            </TreeItem>
            <TreeItem value="recipes" label="Recipes" />
        </Tree>
    );
}
