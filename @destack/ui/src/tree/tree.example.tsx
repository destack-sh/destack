import { defineExample } from "@destack/package/declare";
import { createSignal } from "@destack/view";
import { Tree, TreeItem } from "./tree.tsx";

/** Notebooks and their notes as a tree, opening a note on selection. */
export const treeNotebooks = defineExample({
    of: Tree,
    name: "notebooks",
    description: "notebooks and their notes as a tree, opening a note on selection",
    render: () => {
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
    },
});

/** Notebooks as a tree with trips collapsed and work expanded, reporting the item a person selects. */
export const treeWorkExpanded = defineExample({
    of: Tree,
    name: "work-expanded",
    description:
        "notebooks as a tree with trips collapsed and work expanded, reporting the item a person selects",
    render: () => {
        const [selected, setSelected] = createSignal("");

        return (
            <>
                <Tree aria-label="Notebooks" onValueChange={setSelected}>
                    <TreeItem value="trips" label="Trips">
                        <TreeItem value="lisbon" label="Lisbon" />
                        <TreeItem value="porto" label="Porto" />
                    </TreeItem>
                    <TreeItem value="work" label="Work" defaultExpanded>
                        <TreeItem value="plans" label="Plans" />
                    </TreeItem>
                    <TreeItem value="recipes" label="Recipes" />
                </Tree>
                <output>{selected()}</output>
            </>
        );
    },
});

/** Notebooks as a tree with the Lisbon note selected inside its expanded notebook. */
export const treeLisbonSelected = defineExample({
    of: Tree,
    name: "lisbon-selected",
    description: "notebooks as a tree with the Lisbon note selected inside its expanded notebook",
    render: () => (
        <Tree aria-label="Notebooks" defaultValue="lisbon">
            <TreeItem value="trips" label="Trips" defaultExpanded>
                <TreeItem value="lisbon" label="Lisbon" />
                <TreeItem value="porto" label="Porto" />
            </TreeItem>
            <TreeItem value="recipes" label="Recipes" />
        </Tree>
    ),
});
