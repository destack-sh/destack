import listChecks from "@destack/icon/phosphor/list-checks";
import type { JSX } from "@destack/view";
import { ActionPalette } from "../../action/index.ts";
import type { PaletteItem } from "../../palette/index.ts";
import { choiceOf, STATUSES, type Studio } from "./state.ts";

/** Render the search over the studio: what a person can do where they are, and its tasks. */
export function Search(properties: {
    /** The studio. */
    readonly studio: Studio;
    /** Open a task. */
    readonly onOpen: (task: string) => void;
}): JSX.Element {
    const tasks = (): readonly PaletteItem[] =>
        properties.studio.tasks().map((task) => ({
            id: task.id,
            title: task.title,
            subtitle: task.id,
            icon: listChecks,
            accessories: [{ text: choiceOf(STATUSES, task.status).title }],
            actions: [{ id: "open", title: "Open", run: () => properties.onOpen(task.id) }],
        }));

    return (
        <ActionPalette
            id="studio"
            title="Harbour Studio"
            groups={[{ id: "tasks", title: "Tasks", items: tasks }]}
        />
    );
}
