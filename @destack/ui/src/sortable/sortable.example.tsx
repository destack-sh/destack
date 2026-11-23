import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { createSignal, For } from "@destack/view";
import { VirtualList } from "../virtual-list/index.ts";
import {
    Sortable,
    SortableContainer,
    SortableHandle,
    SortableItem,
    SortableMove,
} from "./sortable.tsx";

/** The height of a long list's row, in pixels. */
const ROW_HEIGHT = 36;

/** The styles of a long list. */
const styles = style.create({
    frame: {
        width: "20rem",
        maxWidth: "100%",
    },
    list: {
        height: "18rem",
    },
    row: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        height: `${String(ROW_HEIGHT)}px`,
    },
});

/** A thousand tasks to order. */
const TASKS = Array.from({ length: 1000 }, (_, index) => `Task ${String(index + 1)}`);

/** A checklist whose steps a drag or the keyboard reorders. */
export const sortableChecklist = defineExample({
    of: Sortable,
    name: "checklist",
    description:
        "a checklist whose steps a drag or the keyboard reorders, the owner placing each moved step between its new neighbours",
    properties: {
        nesting: false,
    },
    render: (properties) => {
        const [steps, setSteps] = createSignal(["Pack", "Check in", "Board"]);

        return (
            <Sortable
                {...properties}
                onMove={(move) => {
                    // place the step after the one it lands after
                    const rest = steps().filter((step) => step !== move.id);
                    const index = move.previous === undefined ? 0 : rest.indexOf(move.previous) + 1;
                    setSteps([...rest.slice(0, index), move.id, ...rest.slice(index)]);
                }}
            >
                <SortableContainer id="steps" label="Steps">
                    <For each={steps()}>
                        {(step) => (
                            <SortableItem id={step}>
                                <SortableHandle /> {step}
                            </SortableItem>
                        )}
                    </For>
                </SortableContainer>
            </Sortable>
        );
    },
});

/** A checklist whose steps stay in their order. */
export const sortableChecklistFixed = defineExample({
    of: Sortable,
    name: "checklist-fixed",
    description: "a checklist whose steps stay in their order",
    render: () => {
        const [steps, setSteps] = createSignal(["Pack", "Check in", "Board"]);

        return (
            <Sortable
                onMove={(move) => {
                    // place the step after the one it lands after
                    const rest = steps().filter((step) => step !== move.id);
                    const index = move.previous === undefined ? 0 : rest.indexOf(move.previous) + 1;
                    setSteps([...rest.slice(0, index), move.id, ...rest.slice(index)]);
                }}
            >
                <SortableContainer id="steps" label="Steps">
                    <For each={steps()}>
                        {(step) => (
                            <SortableItem id={step} disabled>
                                <SortableHandle /> {step}
                            </SortableItem>
                        )}
                    </For>
                </SortableContainer>
            </Sortable>
        );
    },
});

/** A board of two columns whose cards move within and between the columns. */
export const sortableBoard = defineExample({
    of: Sortable,
    name: "board",
    description: "a board of two columns whose cards move within and between the columns",
    render: () => (
        <Sortable onMove={() => undefined}>
            <SortableContainer id="todo" label="To do">
                <SortableItem id="draft">
                    <SortableHandle /> Draft the post
                </SortableItem>
                <SortableItem id="review">
                    <SortableHandle /> Review the post
                </SortableItem>
            </SortableContainer>
            <SortableContainer id="done" label="Done">
                <SortableItem id="outline">
                    <SortableHandle /> Outline the post
                </SortableItem>
            </SortableContainer>
        </Sortable>
    ),
});

/** A thousand tasks in a virtual list that mounts only those in view, ordered by drags and keys. */
export const sortableLongList = defineExample({
    of: Sortable,
    name: "long-list",
    description:
        "a thousand tasks in a virtual list whose rows mount only in view, reordered by drags and keys",
    render: () => {
        // follow the tasks' order
        const [tasks, setTasks] = createSignal(TASKS);

        return (
            <Sortable
                xstyle={styles.frame}
                onMove={(move) =>
                    setTasks((current) =>
                        SortableMove.place(current, move.id, move, (task) => task),
                    )
                }
            >
                <SortableContainer id="tasks" label="Tasks">
                    <VirtualList
                        each={tasks()}
                        estimateSize={() => ROW_HEIGHT}
                        itemKey={(task) => task}
                        itemProperties={(_, index) => ({
                            role: "listitem",
                            "aria-posinset": index + 1,
                            "aria-setsize": tasks().length,
                        })}
                        role="list"
                        aria-label="Tasks"
                        xstyle={styles.list}
                    >
                        {(task) => (
                            <SortableItem id={task} xstyle={styles.row}>
                                <SortableHandle /> {task}
                            </SortableItem>
                        )}
                    </VirtualList>
                </SortableContainer>
            </Sortable>
        );
    },
});
